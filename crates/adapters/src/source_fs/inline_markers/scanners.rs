use crate::source_fs::scanner::is_quote_delimiter;

pub(super) fn first_placeholder_index(source: &str, start: usize, end: usize) -> Option<usize> {
    PlaceholderScanner::new(source, start, end).next_placeholder_index()
}

pub(super) fn placeholder_count(source: &str) -> usize {
    PlaceholderScanner::new(source, 0, source.len()).count()
}

pub(super) fn first_statement_separator_index(
    source: &str,
    start: usize,
    end: usize,
) -> Option<usize> {
    StatementSeparatorScanner::new(source, start, end).next_separator_index()
}

struct PlaceholderScanner<'a> {
    source: &'a str,
    index: usize,
    end: usize,
}

impl<'a> PlaceholderScanner<'a> {
    const fn new(source: &'a str, start: usize, end: usize) -> Self {
        Self {
            source,
            index: start,
            end,
        }
    }

    fn count(mut self) -> usize {
        let mut count = 0;
        while self.next_placeholder_index().is_some() {
            count += 1;
        }

        count
    }

    fn next_placeholder_index(&mut self) -> Option<usize> {
        while !self.is_at_end() {
            if self.starts_with("/*") {
                self.skip_block_comment();
            } else if self.is_line_comment_start() {
                self.skip_line_comment();
            } else if self.current_char().is_some_and(is_quote_delimiter) {
                self.skip_quoted();
            } else if self.current_char() == Some('?') {
                let index = self.index;
                self.advance_current();
                return Some(index);
            } else {
                self.advance_current();
            }
        }

        None
    }

    fn skip_block_comment(&mut self) {
        self.advance_current();
        self.advance_current();

        while !self.is_at_end() {
            if self.starts_with("*/") {
                self.advance_current();
                self.advance_current();
                return;
            }

            self.advance_current();
        }
    }

    fn skip_line_comment(&mut self) {
        while let Some(char) = self.advance_current() {
            if char == '\n' {
                return;
            }
        }
    }

    fn skip_quoted(&mut self) {
        let delimiter = self
            .current_char()
            .expect("quoted skip should start at a delimiter");
        self.advance_current();

        while let Some(char) = self.current_char() {
            self.advance_current();

            if delimiter != '`' && char == '\\' {
                if !self.is_at_end() {
                    self.advance_current();
                }
                continue;
            }

            if char == delimiter {
                if self.current_char() == Some(delimiter) {
                    self.advance_current();
                } else {
                    break;
                }
            }
        }
    }

    fn advance_current(&mut self) -> Option<char> {
        let char = self.current_char()?;
        self.index += char.len_utf8();
        Some(char)
    }

    fn current_char(&self) -> Option<char> {
        if self.is_at_end() {
            return None;
        }

        self.source[self.index..self.end].chars().next()
    }

    const fn is_at_end(&self) -> bool {
        self.index >= self.end
    }

    fn starts_with(&self, needle: &str) -> bool {
        self.source[self.index..self.end].starts_with(needle)
    }

    fn is_line_comment_start(&self) -> bool {
        self.starts_with("#")
            || (self.starts_with("--")
                && self.source[self.index + 2..self.end]
                    .chars()
                    .next()
                    .is_none_or(char::is_whitespace))
    }
}

struct StatementSeparatorScanner<'a> {
    source: &'a str,
    index: usize,
    end: usize,
}

impl<'a> StatementSeparatorScanner<'a> {
    const fn new(source: &'a str, start: usize, end: usize) -> Self {
        Self {
            source,
            index: start,
            end,
        }
    }

    fn next_separator_index(&mut self) -> Option<usize> {
        while !self.is_at_end() {
            if self.starts_with("/*") {
                self.skip_block_comment();
            } else if self.is_line_comment_start() {
                self.skip_line_comment();
            } else if self.current_char().is_some_and(is_quote_delimiter) {
                self.skip_quoted();
            } else if self.current_char() == Some(';') {
                let index = self.index;
                self.advance_current();
                return Some(index);
            } else {
                self.advance_current();
            }
        }

        None
    }

    fn skip_block_comment(&mut self) {
        self.advance_current();
        self.advance_current();

        while !self.is_at_end() {
            if self.starts_with("*/") {
                self.advance_current();
                self.advance_current();
                return;
            }

            self.advance_current();
        }
    }

    fn skip_line_comment(&mut self) {
        while let Some(char) = self.advance_current() {
            if char == '\n' {
                return;
            }
        }
    }

    fn skip_quoted(&mut self) {
        let delimiter = self
            .current_char()
            .expect("quoted skip should start at a delimiter");
        self.advance_current();

        while let Some(char) = self.current_char() {
            self.advance_current();

            if delimiter != '`' && char == '\\' {
                if !self.is_at_end() {
                    self.advance_current();
                }
                continue;
            }

            if char == delimiter {
                if self.current_char() == Some(delimiter) {
                    self.advance_current();
                } else {
                    break;
                }
            }
        }
    }

    fn advance_current(&mut self) -> Option<char> {
        let char = self.current_char()?;
        self.index += char.len_utf8();
        Some(char)
    }

    fn current_char(&self) -> Option<char> {
        if self.is_at_end() {
            return None;
        }

        self.source[self.index..self.end].chars().next()
    }

    const fn is_at_end(&self) -> bool {
        self.index >= self.end
    }

    fn starts_with(&self, needle: &str) -> bool {
        self.source[self.index..self.end].starts_with(needle)
    }

    fn is_line_comment_start(&self) -> bool {
        self.starts_with("#")
            || (self.starts_with("--")
                && self.source[self.index + 2..self.end]
                    .chars()
                    .next()
                    .is_none_or(char::is_whitespace))
    }
}
