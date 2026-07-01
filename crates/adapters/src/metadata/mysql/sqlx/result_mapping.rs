use sqlay_core as core;

/// Map one `MySQL` result column description into core metadata.
#[must_use]
pub fn map_mysql_result_column_metadata(
    name: &str,
    type_name: &str,
    nullable: Option<bool>,
) -> core::DbResultColumn {
    core::DbResultColumn::new_type_ref(
        name.to_owned(),
        mysql_type_name_to_core_type_ref(type_name),
        nullable,
    )
}

pub(super) fn mysql_type_name_to_core_type_ref(type_name: &str) -> core::CoreTypeRef {
    if let Some(values) = parse_mysql_enum_column_type(type_name)
        && let Some(type_ref) = core::CoreTypeRef::from_enum_values(values)
    {
        return type_ref;
    }

    core::CoreTypeRef::from(mysql_type_name_to_core_type(type_name))
}

pub(super) fn mysql_type_name_to_core_type(type_name: &str) -> core::CoreType {
    let normalized = normalized_mysql_type_name(type_name);
    let (base_type, is_unsigned) = normalized
        .strip_suffix(" UNSIGNED")
        .map_or((normalized.as_str(), false), |base_type| (base_type, true));

    match base_type {
        "BOOL" | "BOOLEAN" => core::CoreType::Bool,
        "INT" | "INTEGER" if is_unsigned => core::CoreType::Int64,
        "TINYINT" | "SMALLINT" | "MEDIUMINT" | "INT" | "INTEGER" => core::CoreType::Int32,
        "BIGINT" if is_unsigned => core::CoreType::Unknown,
        "BIGINT" => core::CoreType::Int64,
        "DEC" | "DECIMAL" | "FIXED" | "NUMERIC" => core::CoreType::Decimal,
        "DOUBLE" | "DOUBLE PRECISION" | "FLOAT" | "REAL" => core::CoreType::Float64,
        "CHAR" | "ENUM" | "LONGTEXT" | "MEDIUMTEXT" | "SET" | "TEXT" | "TINYTEXT" | "VARCHAR" => {
            core::CoreType::String
        }
        "BINARY" | "BLOB" | "LONGBLOB" | "MEDIUMBLOB" | "TINYBLOB" | "VARBINARY" => {
            core::CoreType::Bytes
        }
        "DATE" => core::CoreType::Date,
        "TIME" => core::CoreType::Time,
        "DATETIME" | "TIMESTAMP" => core::CoreType::DateTime,
        "JSON" => core::CoreType::Json,
        _ => core::CoreType::Unknown,
    }
}

fn parse_mysql_enum_column_type(type_name: &str) -> Option<Vec<String>> {
    let trimmed = type_name.trim();
    if !trimmed
        .get(..5)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("enum("))
        || !trimmed.ends_with(')')
    {
        return None;
    }

    let body = &trimmed[5..trimmed.len() - 1];
    parse_mysql_quoted_string_list(body)
}

fn parse_mysql_quoted_string_list(body: &str) -> Option<Vec<String>> {
    let mut values = Vec::new();
    let mut chars = body.chars().peekable();

    loop {
        skip_ascii_whitespace(&mut chars);
        if chars.peek().is_none() {
            break;
        }
        if chars.next()? != '\'' {
            return None;
        }

        let mut value = String::new();
        loop {
            match chars.next()? {
                '\\' => value.push(chars.next()?),
                '\'' if chars.peek() == Some(&'\'') => {
                    chars.next();
                    value.push('\'');
                }
                '\'' => break,
                character => value.push(character),
            }
        }
        values.push(value);

        skip_ascii_whitespace(&mut chars);
        match chars.peek() {
            Some(',') => {
                chars.next();
                skip_ascii_whitespace(&mut chars);
                chars.peek()?;
            }
            Some(_) => return None,
            None => break,
        }
    }

    if values.is_empty() {
        return None;
    }

    Some(values)
}

fn skip_ascii_whitespace(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) {
    while chars.peek().is_some_and(char::is_ascii_whitespace) {
        chars.next();
    }
}

fn normalized_mysql_type_name(type_name: &str) -> String {
    let mut without_precision = String::with_capacity(type_name.len());
    let mut precision_depth = 0_u8;

    for character in type_name.trim().chars() {
        match character {
            '(' => precision_depth = precision_depth.saturating_add(1),
            ')' if precision_depth > 0 => precision_depth -= 1,
            _ if precision_depth == 0 => without_precision.push(character),
            _ => {}
        }
    }

    let mut collapsed = String::with_capacity(without_precision.len());
    for word in without_precision.split_whitespace() {
        if !collapsed.is_empty() {
            collapsed.push(' ');
        }
        collapsed.push_str(word);
    }

    collapsed.to_ascii_uppercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mysql_enum_column_type_maps_to_ordered_enum_type_ref() {
        let type_ref =
            mysql_type_name_to_core_type_ref(r"enum('draft','needs\'review','can''ship')");

        assert_eq!(type_ref.core_type(), core::CoreType::String);
        assert_eq!(
            type_ref.enum_values(),
            Some(
                [
                    "draft".to_owned(),
                    "needs'review".to_owned(),
                    "can'ship".to_owned()
                ]
                .as_slice()
            )
        );
    }

    #[test]
    fn malformed_mysql_enum_column_type_falls_back_to_scalar_string() {
        let type_ref = mysql_type_name_to_core_type_ref("enum('draft',paid)");

        assert_eq!(type_ref, core::CoreTypeRef::from(core::CoreType::String));
        assert_eq!(type_ref.enum_values(), None);

        let type_ref = mysql_type_name_to_core_type_ref("enum('draft',)");

        assert_eq!(type_ref, core::CoreTypeRef::from(core::CoreType::String));
        assert_eq!(type_ref.enum_values(), None);
    }

    #[test]
    fn mysql_set_column_type_stays_scalar_string() {
        let type_ref = mysql_type_name_to_core_type_ref("set('read','write')");

        assert_eq!(type_ref, core::CoreTypeRef::from(core::CoreType::String));
        assert_eq!(type_ref.enum_values(), None);
    }
}
