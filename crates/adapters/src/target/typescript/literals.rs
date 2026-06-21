use std::fmt::Write as _;

/// Render text as a TypeScript double-quoted string literal.
#[must_use]
pub fn typescript_string_literal(value: &str) -> String {
    let mut literal = String::with_capacity(value.len() + 2);
    literal.push('"');

    for ch in value.chars() {
        match ch {
            '"' => literal.push_str("\\\""),
            '\\' => literal.push_str("\\\\"),
            '\n' => literal.push_str("\\n"),
            '\r' => literal.push_str("\\r"),
            '\t' => literal.push_str("\\t"),
            '\u{0008}' => literal.push_str("\\b"),
            '\u{000c}' => literal.push_str("\\f"),
            '\u{2028}' => literal.push_str("\\u2028"),
            '\u{2029}' => literal.push_str("\\u2029"),
            control if control.is_control() => {
                let code_point = u32::from(control);
                write!(&mut literal, "\\u{code_point:04X}").expect("writing to String cannot fail");
            }
            other => literal.push(other),
        }
    }

    literal.push('"');
    literal
}
