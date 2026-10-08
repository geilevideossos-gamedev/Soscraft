use crate::text::Text;

#[derive(Clone, Debug, PartialEq)]
pub struct CommandError {
    pub text: Text,
    pub cursor: Option<usize>,
}

impl CommandError {
    pub fn new(key: &str, args: Vec<Text>) -> Self {
        Self {
            text: Text::translate(key, args),
            cursor: None,
        }
    }

    pub fn at(mut self, cursor: usize) -> Self {
        self.cursor = Some(cursor);
        self
    }
}

pub fn literal(value: impl ToString) -> Text {
    Text::literal(value.to_string())
}

pub fn allowed_in_unquoted(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | '+')
}

fn allowed_in_number(c: char) -> bool {
    c.is_ascii_digit() || c == '.' || c == '-'
}

#[derive(Clone, Debug)]
pub struct StringReader<'a> {
    input: &'a str,
    pub cursor: usize,
}

impl<'a> StringReader<'a> {
    pub fn new(input: &'a str) -> Self {
        Self::at(input, 0)
    }

    pub fn at(input: &'a str, cursor: usize) -> Self {
        Self { input, cursor }
    }

    pub fn input(&self) -> &'a str {
        self.input
    }

    pub fn remaining(&self) -> &'a str {
        &self.input[self.cursor..]
    }

    pub fn can_read(&self) -> bool {
        self.cursor < self.input.len()
    }

    pub fn peek(&self) -> Option<char> {
        self.remaining().chars().next()
    }

    pub fn skip(&mut self) {
        if let Some(c) = self.peek() {
            self.cursor += c.len_utf8();
        }
    }

    pub fn skip_ws(&mut self) {
        while self.peek().is_some_and(char::is_whitespace) {
            self.skip();
        }
    }

    fn take_while(&mut self, keep: impl Fn(char) -> bool) -> &'a str {
        let start = self.cursor;
        while self.peek().is_some_and(&keep) {
            self.skip();
        }
        &self.input[start..self.cursor]
    }

    pub fn read_word(&mut self) -> &'a str {
        self.take_while(|c| c != ' ')
    }

    pub fn read_unquoted(&mut self) -> &'a str {
        self.take_while(allowed_in_unquoted)
    }

    pub fn read_string(&mut self) -> Result<String, CommandError> {
        match self.peek() {
            Some(quote @ ('"' | '\'')) => {
                self.skip();
                self.read_until(quote)
            }
            _ => Ok(self.read_unquoted().to_owned()),
        }
    }

    fn read_until(&mut self, end: char) -> Result<String, CommandError> {
        let mut out = String::new();
        let mut escaped = false;
        while let Some(c) = self.peek() {
            self.skip();
            if escaped {
                if c == end || c == '\\' {
                    out.push(c);
                    escaped = false;
                } else {
                    self.cursor -= c.len_utf8();
                    return Err(
                        CommandError::new("parsing.quote.escape", vec![literal(c)]).at(self.cursor)
                    );
                }
            } else if c == '\\' {
                escaped = true;
            } else if c == end {
                return Ok(out);
            } else {
                out.push(c);
            }
        }
        Err(CommandError::new("parsing.quote.expected.end", Vec::new()).at(self.cursor))
    }

    pub fn read_int(&mut self) -> Result<i32, CommandError> {
        let start = self.cursor;
        let number = self.take_while(allowed_in_number);
        if number.is_empty() {
            return Err(CommandError::new("parsing.int.expected", Vec::new()).at(self.cursor));
        }
        number.parse().map_err(|_| {
            self.cursor = start;
            CommandError::new("parsing.int.invalid", vec![literal(number)]).at(start)
        })
    }

    pub fn read_float(&mut self) -> Result<f32, CommandError> {
        let start = self.cursor;
        let number = self.take_while(allowed_in_number);
        if number.is_empty() {
            return Err(CommandError::new("parsing.float.expected", Vec::new()).at(self.cursor));
        }
        number
            .parse::<f32>()
            .ok()
            .filter(|value| value.is_finite())
            .ok_or_else(|| {
                self.cursor = start;
                CommandError::new("parsing.float.invalid", vec![literal(number)]).at(start)
            })
    }

    pub fn read_double(&mut self) -> Result<f64, CommandError> {
        let start = self.cursor;
        let number = self.take_while(allowed_in_number);
        if number.is_empty() {
            return Err(CommandError::new("parsing.double.expected", Vec::new()).at(self.cursor));
        }
        number
            .parse::<f64>()
            .ok()
            .filter(|value| value.is_finite())
            .ok_or_else(|| {
                self.cursor = start;
                CommandError::new("parsing.double.invalid", vec![literal(number)]).at(start)
            })
    }

    pub fn expect(&mut self, c: char) -> Result<(), CommandError> {
        if self.peek() == Some(c) {
            self.skip();
            Ok(())
        } else {
            Err(CommandError::new("parsing.expected", vec![literal(c)]).at(self.cursor))
        }
    }

    pub fn read_bool(&mut self) -> Result<bool, CommandError> {
        let start = self.cursor;
        let value = self.read_string()?;
        match value.as_str() {
            "" => Err(CommandError::new("parsing.bool.expected", Vec::new()).at(self.cursor)),
            "true" => Ok(true),
            "false" => Ok(false),
            _ => {
                self.cursor = start;
                Err(CommandError::new("parsing.bool.invalid", vec![literal(value)]).at(start))
            }
        }
    }

    pub fn read_bounded_int(&mut self, min: i32, max: i32) -> Result<i32, CommandError> {
        let start = self.cursor;
        let value = self.read_int()?;
        if value < min {
            self.cursor = start;
            return Err(CommandError::new(
                "argument.integer.low",
                vec![literal(min), literal(value)],
            )
            .at(start));
        }
        if value > max {
            self.cursor = start;
            return Err(CommandError::new(
                "argument.integer.big",
                vec![literal(max), literal(value)],
            )
            .at(start));
        }
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(error: &CommandError) -> &str {
        match &error.text.content {
            crate::text::Content::Translate { key, .. } => key,
            _ => "",
        }
    }

    #[test]
    fn words_ints_and_quoted_strings_parse_with_cursors() {
        let mut reader = StringReader::new("time set 1000 \"a \\\"b\\\"\" 'c' true");
        assert_eq!(reader.read_unquoted(), "time");
        reader.skip();
        assert_eq!(reader.read_word(), "set");
        reader.skip();
        assert_eq!(reader.read_int(), Ok(1000));
        assert_eq!(reader.cursor, 13);
        reader.skip_ws();
        assert_eq!(reader.read_string(), Ok("a \"b\"".to_owned()));
        reader.skip();
        assert_eq!(reader.read_string(), Ok("c".to_owned()));
        reader.skip();
        assert_eq!(reader.read_bool(), Ok(true));
        assert!(!reader.can_read());
        let mut unquoted = StringReader::new("minecraft:noon");
        assert_eq!(unquoted.read_unquoted(), "minecraft");
        assert_eq!(unquoted.peek(), Some(':'));
        let mut float = StringReader::new("1.5d");
        assert_eq!(float.read_float(), Ok(1.5));
        assert_eq!(float.remaining(), "d");
    }

    #[test]
    fn bad_numbers_and_booleans_report_vanilla_keys_at_their_start() {
        let mut reader = StringReader::new("x 1-2 maybe \"open");
        let error = reader.read_int().unwrap_err();
        assert_eq!(key(&error), "parsing.int.expected");
        assert_eq!(error.cursor, Some(0));
        reader.cursor = 2;
        let error = reader.read_int().unwrap_err();
        assert_eq!(key(&error), "parsing.int.invalid");
        assert_eq!((error.cursor, reader.cursor), (Some(2), 2));
        reader.cursor = 6;
        let error = reader.read_bool().unwrap_err();
        assert_eq!(key(&error), "parsing.bool.invalid");
        assert_eq!(error.cursor, Some(6));
        reader.cursor = 12;
        let error = reader.read_string().unwrap_err();
        assert_eq!(key(&error), "parsing.quote.expected.end");
    }

    #[test]
    fn integer_bounds_report_low_and_big() {
        let low = StringReader::new("-1").read_bounded_int(0, 8).unwrap_err();
        assert_eq!(key(&low), "argument.integer.low");
        assert_eq!(
            low.text,
            Text::translate("argument.integer.low", vec![literal(0), literal(-1)])
        );
        let big = StringReader::new("9").read_bounded_int(0, 8).unwrap_err();
        assert_eq!(key(&big), "argument.integer.big");
        assert_eq!(StringReader::new("8").read_bounded_int(0, 8), Ok(8));
    }
}
