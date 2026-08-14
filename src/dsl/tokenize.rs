use super::{DslError, Location};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TokenKind {
    Keyword,
    Identifier,
    Surface,
    Number,
    String,
    Color,
    Punctuation,
    Operator,
    Eof,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub lexeme: String,
    pub number_value: Option<f64>,
    pub string_value: Option<String>,
    pub source_name: String,
    pub line: usize,
    pub column: usize,
    pub index: usize,
}

impl Token {
    #[must_use]
    pub fn location(&self) -> Location {
        Location {
            source_name: self.source_name.clone(),
            line: self.line,
            column: self.column,
            index: self.index,
        }
    }
}

pub fn tokenize_dsl(source: &str, source_name: &str) -> Result<Vec<Token>, DslError> {
    let mut scanner = Scanner {
        source,
        source_name,
        index: 0,
        line: 1,
        column: 1,
        tokens: Vec::new(),
    };
    scanner.scan()?;
    Ok(scanner.tokens)
}

struct Scanner<'a> {
    source: &'a str,
    source_name: &'a str,
    index: usize,
    line: usize,
    column: usize,
    tokens: Vec<Token>,
}

impl Scanner<'_> {
    fn scan(&mut self) -> Result<(), DslError> {
        while let Some(character) = self.peek(0) {
            if character.is_whitespace() {
                self.advance();
                continue;
            }
            if character == '/' && self.peek(1) == Some('/') {
                while self.peek(0).is_some_and(|character| character != '\n') {
                    self.advance();
                }
                continue;
            }
            if character == '/' && self.peek(1) == Some('*') {
                let location = self.location();
                self.advance();
                self.advance();
                while self.peek(0).is_some()
                    && !(self.peek(0) == Some('*') && self.peek(1) == Some('/'))
                {
                    self.advance();
                }
                if self.peek(0).is_none() {
                    return Err(DslError::new("Unterminated block comment", &location));
                }
                self.advance();
                self.advance();
                continue;
            }
            let location = self.location();
            if character == '#' {
                let start = self.index;
                self.advance();
                while self
                    .peek(0)
                    .is_some_and(|character| character.is_ascii_hexdigit())
                {
                    self.advance();
                }
                let lexeme = &self.source[start..self.index];
                if !matches!(lexeme.len(), 4 | 7 | 9) {
                    return Err(DslError::new(
                        "Colors must use #RGB, #RRGGBB, or #RRGGBBAA",
                        &location,
                    ));
                }
                self.push(TokenKind::Color, lexeme.into(), None, None, &location);
                continue;
            }
            if character == '"' {
                let start = self.index;
                self.advance();
                let mut value = String::new();
                while let Some(next) = self.peek(0) {
                    if next == '"' {
                        break;
                    }
                    if next == '\n' {
                        return Err(DslError::new("Unterminated string", &location));
                    }
                    if next == '\\' {
                        self.advance();
                        let Some(escaped) = self.advance() else {
                            return Err(DslError::new("Unterminated string", &location));
                        };
                        value.push(match escaped {
                            'n' => '\n',
                            't' => '\t',
                            value => value,
                        });
                    } else {
                        value.push(self.advance().unwrap());
                    }
                }
                if self.peek(0).is_none() {
                    return Err(DslError::new("Unterminated string", &location));
                }
                self.advance();
                let lexeme = self.source[start..self.index].to_string();
                self.push(TokenKind::String, lexeme, None, Some(value), &location);
                continue;
            }
            if character.is_ascii_digit()
                || (character == '.' && self.peek(1).is_some_and(|value| value.is_ascii_digit()))
            {
                let start = self.index;
                while self.peek(0).is_some_and(|value| value.is_ascii_digit()) {
                    self.advance();
                }
                if self.peek(0) == Some('.') {
                    self.advance();
                    while self.peek(0).is_some_and(|value| value.is_ascii_digit()) {
                        self.advance();
                    }
                }
                if matches!(self.peek(0), Some('e' | 'E')) {
                    self.advance();
                    if matches!(self.peek(0), Some('+' | '-')) {
                        self.advance();
                    }
                    if !self.peek(0).is_some_and(|value| value.is_ascii_digit()) {
                        return Err(DslError::new("Invalid number", &location));
                    }
                    while self.peek(0).is_some_and(|value| value.is_ascii_digit()) {
                        self.advance();
                    }
                }
                let lexeme = &self.source[start..self.index];
                let value = lexeme
                    .parse::<f64>()
                    .map_err(|_| DslError::new("Invalid number", &location))?;
                self.push(
                    TokenKind::Number,
                    lexeme.into(),
                    Some(value),
                    None,
                    &location,
                );
                continue;
            }
            if character.is_ascii_alphabetic() || character == '_' {
                let start = self.index;
                self.advance();
                while self
                    .peek(0)
                    .is_some_and(|value| value.is_ascii_alphanumeric() || value == '_')
                {
                    self.advance();
                }
                let lexeme = &self.source[start..self.index];
                let kind = if lexeme.starts_with('o')
                    && lexeme.len() > 1
                    && lexeme[1..].bytes().all(|byte| byte.is_ascii_digit())
                {
                    TokenKind::Surface
                } else if matches!(lexeme, "search" | "let" | "render" | "true" | "false") {
                    TokenKind::Keyword
                } else {
                    TokenKind::Identifier
                };
                self.push(kind, lexeme.into(), None, None, &location);
                continue;
            }
            let kind = if "()[],.:=;".contains(character) {
                Some(TokenKind::Punctuation)
            } else if "+-*/".contains(character) {
                Some(TokenKind::Operator)
            } else {
                None
            };
            if let Some(kind) = kind {
                self.advance();
                self.push(kind, character.to_string(), None, None, &location);
                continue;
            }
            return Err(DslError::new(
                format!(
                    "Unexpected character {}",
                    serde_json::to_string(&character.to_string()).unwrap()
                ),
                &location,
            ));
        }
        let location = self.location();
        self.push(TokenKind::Eof, String::new(), None, None, &location);
        Ok(())
    }

    fn peek(&self, offset: usize) -> Option<char> {
        self.source[self.index..].chars().nth(offset)
    }
    fn advance(&mut self) -> Option<char> {
        let character = self.peek(0)?;
        self.index += character.len_utf8();
        if character == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
        Some(character)
    }
    fn location(&self) -> Location {
        Location {
            source_name: self.source_name.into(),
            line: self.line,
            column: self.column,
            index: self.index,
        }
    }
    fn push(
        &mut self,
        kind: TokenKind,
        lexeme: String,
        number_value: Option<f64>,
        string_value: Option<String>,
        location: &Location,
    ) {
        self.tokens.push(Token {
            kind,
            lexeme,
            number_value,
            string_value,
            source_name: location.source_name.clone(),
            line: location.line,
            column: location.column,
            index: location.index,
        });
    }
}
