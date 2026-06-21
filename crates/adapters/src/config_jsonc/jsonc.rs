pub(super) fn normalize_jsonc(source: &str) -> Result<String, &'static str> {
    let without_comments = strip_jsonc_comments(source)?;
    Ok(remove_trailing_commas(&without_comments))
}

fn strip_jsonc_comments(source: &str) -> Result<String, &'static str> {
    let mut stripped = String::with_capacity(source.len());
    let mut chars = source.chars().peekable();
    let mut in_string = false;
    let mut escaped = false;

    while let Some(char) = chars.next() {
        if in_string {
            stripped.push(char);
            if escaped {
                escaped = false;
            } else if char == '\\' {
                escaped = true;
            } else if char == '"' {
                in_string = false;
            }
            continue;
        }

        if char == '"' {
            in_string = true;
            stripped.push(char);
            continue;
        }

        if char == '/' {
            match chars.peek().copied() {
                Some('/') => {
                    chars.next();
                    stripped.push(' ');
                    stripped.push(' ');
                    strip_line_comment(&mut chars, &mut stripped);
                }
                Some('*') => {
                    chars.next();
                    stripped.push(' ');
                    stripped.push(' ');
                    strip_block_comment(&mut chars, &mut stripped)?;
                }
                _ => stripped.push(char),
            }
        } else {
            stripped.push(char);
        }
    }

    Ok(stripped)
}

fn strip_line_comment(chars: &mut std::iter::Peekable<std::str::Chars<'_>>, stripped: &mut String) {
    for char in chars.by_ref() {
        if char == '\n' {
            stripped.push('\n');
            break;
        }

        stripped.push(if char == '\r' { '\r' } else { ' ' });
    }
}

fn strip_block_comment(
    chars: &mut std::iter::Peekable<std::str::Chars<'_>>,
    stripped: &mut String,
) -> Result<(), &'static str> {
    while let Some(char) = chars.next() {
        if char == '*' && chars.peek().copied() == Some('/') {
            chars.next();
            stripped.push(' ');
            stripped.push(' ');
            return Ok(());
        }

        stripped.push(if matches!(char, '\n' | '\r') {
            char
        } else {
            ' '
        });
    }

    Err("unterminated block comment")
}

fn remove_trailing_commas(source: &str) -> String {
    let chars = source.chars().collect::<Vec<_>>();
    let mut normalized = String::with_capacity(source.len());
    let mut index = 0;
    let mut in_string = false;
    let mut escaped = false;

    while index < chars.len() {
        let char = chars[index];

        if in_string {
            normalized.push(char);
            if escaped {
                escaped = false;
            } else if char == '\\' {
                escaped = true;
            } else if char == '"' {
                in_string = false;
            }
            index += 1;
            continue;
        }

        if char == '"' {
            in_string = true;
            normalized.push(char);
            index += 1;
            continue;
        }

        if char == ','
            && next_significant_char(&chars, index + 1)
                .is_some_and(|next| matches!(next, '}' | ']'))
        {
            normalized.push(' ');
        } else {
            normalized.push(char);
        }

        index += 1;
    }

    normalized
}

fn next_significant_char(chars: &[char], start: usize) -> Option<char> {
    chars
        .iter()
        .skip(start)
        .copied()
        .find(|char| !char.is_whitespace())
}
