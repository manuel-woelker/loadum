use loadum::result::LoadumResult;

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum Token {
    Initial,
    StringDoubleQuoted,
    StringSingleQuoted,
    StringPlain,
    EOF,
}

#[derive(Debug)]
pub struct Tokenizer<'source> {
    scanner: unscanny::Scanner<'source>,
    start: usize,
    end: usize,
    current: Token,
}

impl<'source> Tokenizer<'source> {
    pub fn current_str(&self) -> &'source str {
        self.scanner.get(self.start..self.end)
    }
    pub fn start(&self) -> usize {
        self.start
    }
    pub fn end(&self) -> usize {
        self.end
    }
}

impl<'source> Tokenizer<'source> {
    pub fn new(source: &'source str) -> Self {
        Self {
            scanner: unscanny::Scanner::new(source),
            start: 0,
            end: 0,
            current: Token::Initial,
        }
    }
}

impl Tokenizer<'_> {
    pub fn current(&self) -> &Token {
        &self.current
    }

    pub fn advance(&mut self) -> LoadumResult<()> {
        self.scanner.eat_whitespace();
        self.start = self.scanner.cursor();
        let Some(c) = self.scanner.eat() else {
            self.current = Token::EOF;
            return Ok(());
        };
        match c {
            '"' => {
                self.current = Token::StringDoubleQuoted;
                self.scanner.eat_until('\"');
                self.scanner.expect('\"');
            }
            '\'' => {
                self.current = Token::StringSingleQuoted;
                self.scanner.eat_until('\'');
                self.scanner.expect('\'');
            }
            _ => {
                self.current = Token::StringPlain;
                self.scanner.eat_until(": ");
            }
        }
        self.end = self.scanner.cursor();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use expect_test::{Expect, expect};
    use std::io::Cursor;
    use std::io::Write;

    fn test_tokenizer(input: &str, expected: Expect) {
        let mut tokenizer = Tokenizer::new(input);
        let mut output = Cursor::new(vec![]);
        while tokenizer.current != Token::EOF {
            tokenizer.advance().unwrap();
            writeln!(
                output,
                "{:?} [{}-{}] {}",
                tokenizer.current(),
                tokenizer.start(),
                tokenizer.end(),
                tokenizer.current_str()
            )
            .unwrap();
        }
        expected.assert_eq(&str::from_utf8(&output.into_inner()).unwrap());
    }

    #[test]
    fn test_string_double_quoted() {
        test_tokenizer(
            "\"foo\"",
            expect![[r#"
                StringDoubleQuoted [0-5] "foo"
                EOF [5-5] 
            "#]],
        );
        test_tokenizer(
            "\"foo\" \"bar\"",
            expect![[r#"
                StringDoubleQuoted [0-5] "foo"
                StringDoubleQuoted [6-11] "bar"
                EOF [11-11] 
            "#]],
        );
    }

    #[test]
    fn test_string_single_quoted() {
        test_tokenizer(
            "'foo'",
            expect![[r#"
                StringSingleQuoted [0-5] 'foo'
                EOF [5-5] 
            "#]],
        );
        test_tokenizer(
            "'foo' 'bar'",
            expect![[r#"
                StringSingleQuoted [0-5] 'foo'
                StringSingleQuoted [6-11] 'bar'
                EOF [11-11] 
            "#]],
        );
    }

    #[test]
    fn test_string_plain() {
        test_tokenizer(
            "foo",
            expect![[r#"
                StringPlain [0-3] foo
                EOF [3-3] 
            "#]],
        );
        test_tokenizer(
            "foo bar",
            expect![[r#"
                StringPlain [0-7] foo bar
                EOF [7-7] 
            "#]],
        );
    }
}
