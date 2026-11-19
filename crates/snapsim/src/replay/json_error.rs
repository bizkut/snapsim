//! Format malformed JSON like CPython's json.JSONDecodeError.
//!
//! serde_json reports byte-oriented parser errors; replaycheck exposes Python's
//! character-oriented message and offsets. Scan only on the error path.

struct Scan {
    chars: Vec<char>,
    index: usize,
}

type Problem = (&'static str, usize);

impl Scan {
    fn whitespace(&mut self) {
        while matches!(self.chars.get(self.index), Some(' ' | '\t' | '\r' | '\n')) {
            self.index += 1;
        }
    }

    fn string(&mut self) -> Result<(), Problem> {
        let start = self.index;
        self.index += 1;
        while let Some(&c) = self.chars.get(self.index) {
            match c {
                '"' => {
                    self.index += 1;
                    return Ok(());
                }
                '\\' => {
                    let escape = self.index;
                    self.index += 1;
                    let Some(&next) = self.chars.get(self.index) else {
                        return Err(("Unterminated string starting at", start));
                    };
                    if next == 'u' {
                        if !(1..=4).all(|n| {
                            self.chars
                                .get(self.index + n)
                                .is_some_and(char::is_ascii_hexdigit)
                        }) {
                            return Err(("Invalid \\uXXXX escape", self.index));
                        }
                        self.index += 5;
                    } else if matches!(next, '"' | '\\' | '/' | 'b' | 'f' | 'n' | 'r' | 't') {
                        self.index += 1;
                    } else {
                        return Err(("Invalid \\escape", escape));
                    }
                }
                '\u{0}'..='\u{1f}' => return Err(("Invalid control character at", self.index)),
                _ => self.index += 1,
            }
        }
        Err(("Unterminated string starting at", start))
    }

    fn value(&mut self, depth: usize) -> Result<(), Problem> {
        self.whitespace();
        let start = self.index;
        if depth > 1024 {
            return Err(("Expecting value", start));
        }
        match self.chars.get(self.index).copied() {
            Some('"') => self.string(),
            Some('[') => self.array(depth + 1),
            Some('{') => self.object(depth + 1),
            Some('-' | '0'..='9') => {
                if self.literal("-Infinity") {
                    return Ok(());
                }
                if self.chars.get(self.index) == Some(&'-') {
                    self.index += 1;
                }
                match self.chars.get(self.index) {
                    Some('0') => self.index += 1,
                    Some('1'..='9') => {
                        self.index += 1;
                        while self.chars.get(self.index).is_some_and(char::is_ascii_digit) {
                            self.index += 1;
                        }
                    }
                    _ => {
                        self.index = start;
                        return Err(("Expecting value", start));
                    }
                }
                if self.chars.get(self.index) == Some(&'.')
                    && self
                        .chars
                        .get(self.index + 1)
                        .is_some_and(char::is_ascii_digit)
                {
                    self.index += 2;
                    while self.chars.get(self.index).is_some_and(char::is_ascii_digit) {
                        self.index += 1;
                    }
                }
                if matches!(self.chars.get(self.index), Some('e' | 'E')) {
                    let mut end = self.index + 1;
                    if matches!(self.chars.get(end), Some('+' | '-')) {
                        end += 1;
                    }
                    if self.chars.get(end).is_some_and(char::is_ascii_digit) {
                        self.index = end + 1;
                        while self.chars.get(self.index).is_some_and(char::is_ascii_digit) {
                            self.index += 1;
                        }
                    }
                }
                Ok(())
            }
            _ if self.literal("true")
                || self.literal("false")
                || self.literal("null")
                || self.literal("NaN")
                || self.literal("Infinity") =>
            {
                Ok(())
            }
            _ => Err(("Expecting value", start)),
        }
    }

    fn literal(&mut self, text: &str) -> bool {
        if self.chars[self.index..]
            .iter()
            .copied()
            .zip(text.chars())
            .all(|(a, b)| a == b)
            && self.chars.len() - self.index >= text.len()
        {
            self.index += text.len();
            true
        } else {
            false
        }
    }

    fn array(&mut self, depth: usize) -> Result<(), Problem> {
        self.index += 1;
        self.whitespace();
        if self.chars.get(self.index) == Some(&']') {
            self.index += 1;
            return Ok(());
        }
        loop {
            self.value(depth)?;
            self.whitespace();
            match self.chars.get(self.index) {
                Some(']') => {
                    self.index += 1;
                    return Ok(());
                }
                Some(',') => {
                    self.index += 1;
                }
                _ => return Err(("Expecting ',' delimiter", self.index)),
            }
        }
    }

    fn object(&mut self, depth: usize) -> Result<(), Problem> {
        self.index += 1;
        self.whitespace();
        if self.chars.get(self.index) == Some(&'}') {
            self.index += 1;
            return Ok(());
        }
        loop {
            self.whitespace();
            if self.chars.get(self.index) != Some(&'"') {
                return Err((
                    "Expecting property name enclosed in double quotes",
                    self.index,
                ));
            }
            self.string()?;
            self.whitespace();
            if self.chars.get(self.index) != Some(&':') {
                return Err(("Expecting ':' delimiter", self.index));
            }
            self.index += 1;
            self.value(depth)?;
            self.whitespace();
            match self.chars.get(self.index) {
                Some('}') => {
                    self.index += 1;
                    return Ok(());
                }
                Some(',') => self.index += 1,
                _ => return Err(("Expecting ',' delimiter", self.index)),
            }
        }
    }
}

pub(crate) fn decode_error(input: &str, error: &serde_json::Error) -> String {
    if input.starts_with('\u{feff}') {
        return "Unexpected UTF-8 BOM (decode using utf-8-sig): line 1 column 1 (char 0)".into();
    }
    let mut scan = Scan {
        chars: input.chars().collect(),
        index: 0,
    };
    let problem = scan.value(0).and_then(|()| {
        scan.whitespace();
        if scan.index == scan.chars.len() {
            Ok(())
        } else {
            Err(("Extra data", scan.index))
        }
    });
    let Err((message, position)) = problem else {
        return error.to_string();
    };
    let prefix = &scan.chars[..position];
    let line = 1 + prefix.iter().filter(|&&c| c == '\n').count();
    let column = 1 + prefix.iter().rev().take_while(|&&c| c != '\n').count();
    format!("{message}: line {line} column {column} (char {position})")
}

#[cfg(test)]
mod tests {
    use super::decode_error;
    #[test]
    fn python_messages_and_character_positions() {
        let cases = [
            (
                "{bad}",
                "Expecting property name enclosed in double quotes: line 1 column 2 (char 1)",
            ),
            ("{\"a\":}", "Expecting value: line 1 column 6 (char 5)"),
            ("[1,]", "Expecting value: line 1 column 4 (char 3)"),
            (
                "{\"a\":1,}",
                "Expecting property name enclosed in double quotes: line 1 column 8 (char 7)",
            ),
            (
                "{\"a\":\"abc}",
                "Unterminated string starting at: line 1 column 6 (char 5)",
            ),
            (
                "{\"a\":\"a\\q\"}",
                "Invalid \\escape: line 1 column 8 (char 7)",
            ),
            (
                "{\"x\": \"é\", \"y\":}",
                "Expecting value: line 1 column 16 (char 15)",
            ),
            ("truex", "Extra data: line 1 column 5 (char 4)"),
        ];
        for (text, expected) in cases {
            let error = serde_json::from_str::<serde_json::Value>(text).unwrap_err();
            assert_eq!(decode_error(text, &error), expected);
        }
    }
}
