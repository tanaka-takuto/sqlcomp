use serde::de::DeserializeOwned;
use serde_json::{Map, Value};
use sqlcomp_core as core;

use crate::source_fs::diagnostics::metadata_error;
use crate::source_fs::scanner::SqlcompBlock;

pub(super) fn parse_sqlcomp_metadata_object(
    block: &SqlcompBlock,
) -> core::DiagnosticResult<Map<String, Value>> {
    let value = parse_sqlcomp_metadata_value(block)?;
    let Value::Object(metadata) = value else {
        return Err(metadata_error(
            "`@sqlcomp` metadata must be an object",
            block.payload_range(),
        ));
    };

    Ok(metadata)
}

pub(super) fn parse_sqlcomp_metadata_value(block: &SqlcompBlock) -> core::DiagnosticResult<Value> {
    match deserialize_sqlcomp_metadata(block) {
        Ok(value) => Ok(value),
        Err(report) => parse_flat_sqlcomp_metadata_value(block.payload()).ok_or(report),
    }
}

pub(super) fn deserialize_sqlcomp_metadata<T>(block: &SqlcompBlock) -> core::DiagnosticResult<T>
where
    T: DeserializeOwned,
{
    match deser_hjson::from_str::<T>(block.payload()) {
        Ok(value) => Ok(value),
        Err(error) => {
            if let Some(normalized) = normalize_inline_hjson_metadata(block.payload())
                && let Ok(value) = deser_hjson::from_str::<T>(&normalized)
            {
                return Ok(value);
            }
            if let Some(value) = parse_flat_sqlcomp_metadata_value(block.payload())
                && let Ok(value) = serde_json::from_value::<T>(value)
            {
                return Ok(value);
            }

            Err(metadata_error(
                format!("failed to parse `@sqlcomp` metadata as Hjson: {error}"),
                block.payload_range(),
            ))
        }
    }
}

fn normalize_inline_hjson_metadata(payload: &str) -> Option<String> {
    let mut normalized = String::with_capacity(payload.len());
    let mut index = 0;
    let mut changed = false;
    let mut in_single_quote = false;
    let mut in_double_quote = false;

    while index < payload.len() {
        let char = payload[index..]
            .chars()
            .next()
            .expect("index should point at a character");

        if !in_single_quote && !in_double_quote && char.is_whitespace() {
            let whitespace_start = index;
            while index < payload.len() {
                let whitespace_char = payload[index..]
                    .chars()
                    .next()
                    .expect("index should point at a character");
                if !whitespace_char.is_whitespace() {
                    break;
                }
                index += whitespace_char.len_utf8();
            }

            let previous_significant = last_non_whitespace_char(&normalized);
            let should_insert_line_break = metadata_key_starts(&payload[index..])
                && !matches!(previous_significant, None | Some(',' | '\n' | '\r'))
                || payload[index..].starts_with('}')
                    && !matches!(previous_significant, None | Some('{' | ',' | '\n' | '\r'));
            if should_insert_line_break {
                normalized.push('\n');
                changed = true;
            } else {
                normalized.push_str(&payload[whitespace_start..index]);
            }
            continue;
        }

        if !in_double_quote && char == '\'' {
            in_single_quote = !in_single_quote;
        } else if !in_single_quote && char == '"' {
            in_double_quote = !in_double_quote;
        }

        normalized.push(char);
        index += char.len_utf8();
    }

    changed.then_some(normalized)
}

fn metadata_key_starts(source: &str) -> bool {
    let mut chars = source.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first == '_' || first.is_ascii_alphabetic()) {
        return false;
    }

    for char in chars {
        if char == ':' {
            return true;
        }
        if !(char == '_' || char.is_ascii_alphanumeric()) {
            return false;
        }
    }

    false
}

fn parse_flat_sqlcomp_metadata_value(payload: &str) -> Option<Value> {
    let normalized = normalize_inline_hjson_metadata(payload)?;
    let trimmed = normalized.trim();
    let inner = trimmed.strip_prefix('{')?.strip_suffix('}')?;
    let mut metadata = Map::new();

    for line in inner.lines() {
        let line = line.trim().trim_end_matches(',');
        if line.is_empty() {
            continue;
        }

        let (key, value) = line.split_once(':')?;
        let key = key.trim();
        if !is_metadata_key(key) {
            return None;
        }

        metadata.insert(key.to_owned(), flat_metadata_value(value.trim())?);
    }

    Some(Value::Object(metadata))
}

fn is_metadata_key(key: &str) -> bool {
    let mut chars = key.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first == '_' || first.is_ascii_alphabetic())
        && chars.all(|char| char == '_' || char.is_ascii_alphanumeric())
}

fn flat_metadata_value(value: &str) -> Option<Value> {
    if value.is_empty() {
        return None;
    }

    if let Some(array) = flat_metadata_array_value(value) {
        return Some(array);
    }

    match value {
        "true" => Some(Value::Bool(true)),
        "false" => Some(Value::Bool(false)),
        _ => Some(Value::String(flat_metadata_string_value(value).to_owned())),
    }
}

fn flat_metadata_array_value(value: &str) -> Option<Value> {
    let inner = value.strip_prefix('[')?.strip_suffix(']')?;
    if inner.trim().is_empty() {
        return Some(Value::Array(Vec::new()));
    }

    inner
        .split(',')
        .map(|item| {
            let item = item.trim();
            (!item.is_empty()).then(|| Value::String(flat_metadata_string_value(item).to_owned()))
        })
        .collect::<Option<Vec<_>>>()
        .map(Value::Array)
}

fn flat_metadata_string_value(value: &str) -> &str {
    value
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .or_else(|| {
            value
                .strip_prefix('\'')
                .and_then(|value| value.strip_suffix('\''))
        })
        .unwrap_or(value)
}

fn last_non_whitespace_char(source: &str) -> Option<char> {
    source.chars().rev().find(|char| !char.is_whitespace())
}
