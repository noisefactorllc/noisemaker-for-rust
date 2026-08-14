use super::{DslError, Location, Token, TokenKind, tokenize_dsl};

#[derive(Clone, Debug, PartialEq)]
pub struct SurfaceRef {
    pub name: String,
    pub loc: Location,
}

#[derive(Clone, Debug, PartialEq)]
pub enum DslValue {
    Number(f64),
    String(String),
    Bool(bool),
    Array {
        values: Vec<DslValue>,
        loc: Location,
    },
    Surface(SurfaceRef),
    Vector {
        width: usize,
        values: Vec<DslValue>,
        loc: Location,
    },
    Identifier {
        name: String,
        loc: Location,
    },
    Unary {
        operator: String,
        argument: Box<DslValue>,
        loc: Location,
    },
    Binary {
        operator: String,
        left: Box<DslValue>,
        right: Box<DslValue>,
        loc: Location,
    },
    Call(Box<DslCall>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct DslArgument {
    pub name: Option<String>,
    pub value: DslValue,
    pub loc: Location,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DslCall {
    pub name: String,
    pub args: Vec<DslArgument>,
    pub arg_mode: Option<String>,
    pub loc: Location,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DslChain {
    pub calls: Vec<DslCall>,
    pub loc: Location,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DslBinding {
    pub name: String,
    pub value: DslValue,
    pub loc: Location,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DslProgram {
    pub search: Vec<String>,
    pub bindings: Vec<DslBinding>,
    pub chains: Vec<DslChain>,
    pub render: Option<SurfaceRef>,
    pub loc: Location,
}

pub fn parse_dsl(source: &str, source_name: &str) -> Result<DslProgram, DslError> {
    Parser::new(tokenize_dsl(source, source_name)?).parse_program()
}

struct Parser {
    tokens: Vec<Token>,
    current: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, current: 0 }
    }
    fn peek(&self, offset: usize) -> &Token {
        &self.tokens[(self.current + offset).min(self.tokens.len() - 1)]
    }
    fn at_end(&self) -> bool {
        self.peek(0).kind == TokenKind::Eof
    }
    fn check(&self, lexeme: &str) -> bool {
        self.peek(0).lexeme == lexeme
    }
    fn take_if(&mut self, lexemes: &[&str]) -> bool {
        if !lexemes.contains(&self.peek(0).lexeme.as_str()) {
            return false;
        }
        self.current += 1;
        true
    }
    fn consume(&mut self, lexeme: &str, message: impl Into<String>) -> Result<Token, DslError> {
        if !self.check(lexeme) {
            return Err(DslError::new(message, &self.peek(0).location()));
        }
        let token = self.peek(0).clone();
        self.current += 1;
        Ok(token)
    }
    fn identifier(&mut self, message: &str) -> Result<Token, DslError> {
        if self.peek(0).kind != TokenKind::Identifier {
            return Err(DslError::new(message, &self.peek(0).location()));
        }
        let token = self.peek(0).clone();
        self.current += 1;
        Ok(token)
    }
    fn parse_program(mut self) -> Result<DslProgram, DslError> {
        let loc = self.peek(0).location();
        let mut program = DslProgram {
            search: Vec::new(),
            bindings: Vec::new(),
            chains: Vec::new(),
            render: None,
            loc,
        };
        if self.take_if(&["search"]) {
            loop {
                let token = self.peek(0).clone();
                if token.kind != TokenKind::Identifier
                    && !(token.kind == TokenKind::Keyword && token.lexeme == "render")
                {
                    return Err(DslError::new(
                        "Expected namespace after search",
                        &token.location(),
                    ));
                }
                self.current += 1;
                program.search.push(token.lexeme);
                if !self.take_if(&[","]) {
                    break;
                }
            }
            self.take_if(&[";"]);
        }
        while !self.at_end() {
            if self.take_if(&[";"]) {
                continue;
            }
            if self.take_if(&["let"]) {
                let start = self.tokens[self.current - 1].location();
                program.bindings.push(self.parse_binding(start)?);
            } else if self.take_if(&["render"]) {
                let start = self.tokens[self.current - 1].location();
                if program.render.is_some() {
                    return Err(DslError::new(
                        "Program may only declare one render surface",
                        &start,
                    ));
                }
                self.consume("(", "Expected \"(\"")?;
                let mut surface = self.parse_surface()?;
                surface.loc = start;
                self.consume(")", "Expected \")\"")?;
                self.take_if(&[";"]);
                program.render = Some(surface);
            } else {
                program.chains.push(self.parse_chain()?);
                self.take_if(&[";"]);
            }
        }
        Ok(program)
    }
    fn parse_binding(&mut self, loc: Location) -> Result<DslBinding, DslError> {
        let name = self.identifier("Expected binding name after let")?;
        self.consume("=", "Expected \"=\"")?;
        let value = if self.peek(0).kind == TokenKind::Identifier && self.peek(1).lexeme == "(" {
            DslValue::Call(Box::new(self.parse_call()?))
        } else {
            self.parse_value_expression(0)?
        };
        self.take_if(&[";"]);
        Ok(DslBinding {
            name: name.lexeme,
            value,
            loc,
        })
    }
    fn parse_chain(&mut self) -> Result<DslChain, DslError> {
        let first = self.parse_call()?;
        let loc = first.loc.clone();
        let mut calls = vec![first];
        while self.take_if(&["."]) {
            calls.push(self.parse_call()?);
        }
        Ok(DslChain { calls, loc })
    }
    fn parse_call(&mut self) -> Result<DslCall, DslError> {
        let name = self.identifier("Expected effect or IO function name")?;
        self.consume("(", "Expected \"(\"")?;
        let mut args = Vec::new();
        let mut mode: Option<String> = None;
        if !self.check(")") {
            loop {
                let is_named =
                    self.peek(0).kind == TokenKind::Identifier && self.peek(1).lexeme == ":";
                let next_mode = if is_named { "named" } else { "positional" };
                if mode.as_deref().is_some_and(|mode| mode != next_mode) {
                    return Err(DslError::new(
                        "Cannot mix positional and named arguments",
                        &self.peek(0).location(),
                    ));
                }
                mode = Some(next_mode.into());
                let arg_name = if is_named {
                    let name = self.identifier("Expected identifier")?.lexeme;
                    self.consume(":", "Expected \":\"")?;
                    Some(name)
                } else {
                    None
                };
                let loc = self.peek(0).location();
                let value = self.parse_value_expression(0)?;
                args.push(DslArgument {
                    name: arg_name,
                    value,
                    loc,
                });
                if !self.take_if(&[","]) {
                    break;
                }
            }
        }
        self.consume(")", "Expected \")\"")?;
        let loc = name.location();
        Ok(DslCall {
            name: name.lexeme,
            args,
            arg_mode: mode,
            loc,
        })
    }
    fn parse_value_expression(&mut self, minimum: u8) -> Result<DslValue, DslError> {
        let mut left = self.parse_value_unary()?;
        loop {
            let precedence = match self.peek(0).lexeme.as_str() {
                "+" | "-" => 1,
                "*" | "/" => 2,
                _ => 0,
            };
            if precedence == 0 || precedence < minimum {
                break;
            }
            let operator = self.peek(0).clone();
            self.current += 1;
            let right = self.parse_value_expression(precedence + 1)?;
            let loc = operator.location();
            left = DslValue::Binary {
                operator: operator.lexeme,
                left: Box::new(left),
                right: Box::new(right),
                loc,
            };
        }
        Ok(left)
    }
    fn parse_value_unary(&mut self) -> Result<DslValue, DslError> {
        if self.take_if(&["-", "+"]) {
            let operator = self.tokens[self.current - 1].clone();
            let loc = operator.location();
            return Ok(DslValue::Unary {
                operator: operator.lexeme,
                argument: Box::new(self.parse_value_unary()?),
                loc,
            });
        }
        self.parse_value_primary()
    }
    fn parse_value_primary(&mut self) -> Result<DslValue, DslError> {
        let token = self.peek(0).clone();
        if token.kind == TokenKind::Number {
            self.current += 1;
            return Ok(DslValue::Number(token.number_value.unwrap()));
        }
        if token.kind == TokenKind::String {
            self.current += 1;
            return Ok(DslValue::String(token.string_value.unwrap()));
        }
        if matches!(token.lexeme.as_str(), "true" | "false") {
            self.current += 1;
            return Ok(DslValue::Bool(token.lexeme == "true"));
        }
        if token.kind == TokenKind::Color {
            self.current += 1;
            return Ok(DslValue::Array {
                values: parse_color(&token.lexeme)
                    .into_iter()
                    .map(DslValue::Number)
                    .collect(),
                loc: token.location(),
            });
        }
        if token.kind == TokenKind::Surface {
            return self.parse_surface().map(DslValue::Surface);
        }
        if self.take_if(&["["]) {
            let loc = token.location();
            let mut values = Vec::new();
            if !self.check("]") {
                loop {
                    values.push(self.parse_value_expression(0)?);
                    if !self.take_if(&[","]) {
                        break;
                    }
                }
            }
            self.consume("]", "Expected \"]\"")?;
            return Ok(DslValue::Array { values, loc });
        }
        if self.take_if(&["("]) {
            let value = self.parse_value_expression(0)?;
            self.consume(")", "Expected \")\"")?;
            return Ok(value);
        }
        if token.kind == TokenKind::Identifier {
            self.current += 1;
            let name = token.lexeme.clone();
            if name == "read" && self.take_if(&["("]) {
                if self.peek(0).kind == TokenKind::Identifier && self.peek(1).lexeme == ":" {
                    let argument_name = self.identifier("Expected identifier")?;
                    if !matches!(argument_name.lexeme.as_str(), "surface" | "tex") {
                        return Err(DslError::new(
                            "read() surface argument must be named \"surface\" or \"tex\"",
                            &argument_name.location(),
                        ));
                    }
                    self.consume(":", "Expected \":\"")?;
                }
                let surface = self.parse_surface()?;
                self.consume(")", "Expected \")\"")?;
                return Ok(DslValue::Surface(surface));
            }
            if matches!(name.as_str(), "vec2" | "vec3" | "vec4") && self.take_if(&["("]) {
                let mut values = Vec::new();
                if !self.check(")") {
                    loop {
                        values.push(self.parse_value_expression(0)?);
                        if !self.take_if(&[","]) {
                            break;
                        }
                    }
                }
                self.consume(")", "Expected \")\"")?;
                return Ok(DslValue::Vector {
                    width: name[3..].parse().unwrap(),
                    values,
                    loc: token.location(),
                });
            }
            let mut path = name;
            while self.take_if(&["."]) {
                let member = self.identifier("Expected enum member")?;
                path.push('.');
                path.push_str(&member.lexeme);
            }
            return Ok(DslValue::Identifier {
                name: path,
                loc: token.location(),
            });
        }
        Err(DslError::new("Expected DSL value", &token.location()))
    }
    fn parse_surface(&mut self) -> Result<SurfaceRef, DslError> {
        let token = self.peek(0).clone();
        if token.kind != TokenKind::Surface {
            return Err(DslError::new(
                "Expected surface reference",
                &token.location(),
            ));
        }
        self.current += 1;
        if !matches!(
            token.lexeme.as_str(),
            "o0" | "o1" | "o2" | "o3" | "o4" | "o5" | "o6" | "o7"
        ) {
            return Err(DslError::new(
                "Surface reference must be o0 through o7",
                &token.location(),
            ));
        }
        let loc = token.location();
        Ok(SurfaceRef {
            name: token.lexeme,
            loc,
        })
    }
}

fn parse_color(lexeme: &str) -> Vec<f64> {
    let mut hex = lexeme[1..].to_string();
    if hex.len() == 3 {
        hex = hex
            .chars()
            .flat_map(|character| [character, character])
            .collect();
    }
    let mut values = [0, 2, 4]
        .into_iter()
        .map(|offset| u8::from_str_radix(&hex[offset..offset + 2], 16).unwrap() as f64 / 255.0)
        .collect::<Vec<_>>();
    if hex.len() == 8 {
        values.push(u8::from_str_radix(&hex[6..8], 16).unwrap() as f64 / 255.0);
    }
    values
}
