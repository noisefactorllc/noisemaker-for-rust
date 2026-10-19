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
    Chain(Box<DslChain>),
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
    pub namespace: Option<String>,
    pub loc: Location,
}

#[derive(Clone, Debug, PartialEq)]
pub enum DslChainElement {
    Call(DslCall),
    Subchain(DslSubchain),
}

#[derive(Clone, Debug, PartialEq)]
pub struct DslSubchain {
    pub name: Option<String>,
    pub id: Option<String>,
    pub body: Vec<DslCall>,
    pub loc: Location,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DslChain {
    pub calls: Vec<DslChainElement>,
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
        // A string token's lexeme is its content, so a string never matches
        // punctuation, an operator or a keyword: `text(")")` holds the string ")".
        self.peek(0).kind != TokenKind::String && self.peek(0).lexeme == lexeme
    }
    fn take_if(&mut self, lexemes: &[&str]) -> bool {
        if self.peek(0).kind == TokenKind::String
            || !lexemes.contains(&self.peek(0).lexeme.as_str())
        {
            return false;
        }
        self.current += 1;
        true
    }
    fn is_punctuation(&self, lexeme: &str) -> bool {
        self.peek(0).kind == TokenKind::Punctuation && self.peek(0).lexeme == lexeme
    }
    fn at_subchain(&self) -> bool {
        self.peek(0).kind == TokenKind::Keyword && self.peek(0).lexeme == "subchain"
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
        let mut calls = vec![DslChainElement::Call(first)];
        while self.take_if(&["."]) {
            if self.at_subchain() {
                calls.push(DslChainElement::Subchain(self.parse_subchain()?));
            } else if self.peek(0).lexeme == "write3d" && self.peek(0).kind == TokenKind::Identifier
            {
                calls.push(DslChainElement::Call(self.parse_write3d()?));
            } else {
                calls.push(DslChainElement::Call(self.parse_call()?));
            }
        }
        Ok(DslChain { calls, loc })
    }
    // Port of upstream parseWriteCall for write3d: two positional references, a
    // volume and a geometry (`write3d(vol0, geo0)`), each an identifier or a surface.
    fn parse_write3d(&mut self) -> Result<DslCall, DslError> {
        let name = self.identifier("Expected effect or IO function name")?;
        self.consume("(", "Expected \"(\"")?;
        let reference = |parser: &mut Self| -> Result<DslValue, DslError> {
            let token = parser.peek(0).clone();
            if token.kind == TokenKind::Surface {
                return parser.parse_surface().map(DslValue::Surface);
            }
            if token.kind != TokenKind::Identifier {
                return Err(DslError::new(
                    "Expected surface reference",
                    &token.location(),
                ));
            }
            parser.current += 1;
            let loc = token.location();
            Ok(DslValue::Identifier {
                name: token.lexeme,
                loc,
            })
        };
        let volume_loc = self.peek(0).location();
        let volume = reference(self)?;
        self.consume(",", "Expected \",\"")?;
        let geometry_loc = self.peek(0).location();
        let geometry = reference(self)?;
        self.consume(")", "Expected \")\"")?;
        let loc = name.location();
        Ok(DslCall {
            name: "write3d".into(),
            args: vec![
                DslArgument {
                    name: None,
                    value: volume,
                    loc: volume_loc,
                },
                DslArgument {
                    name: None,
                    value: geometry,
                    loc: geometry_loc,
                },
            ],
            arg_mode: Some("positional".into()),
            namespace: None,
            loc,
        })
    }
    // Port of upstream parseSubchainCall: `subchain(name: "...", id: "...") { .a() .b() }`.
    // A single leading string is shorthand for `name`. Unknown keys, duplicate keys and
    // a missing ',' between keys are upstream warnings (P008-P010): the program still
    // compiles, an unknown key's value is discarded and the last duplicate wins. The
    // body holds one or more '.'-prefixed calls and cannot hold write(), write3d() or
    // another subchain, which upstream's lexer reserves as keywords.
    fn parse_subchain(&mut self) -> Result<DslSubchain, DslError> {
        let start = self.tokens[self.current].clone();
        self.current += 1;
        self.consume("(", "Expected \"(\"")?;
        let mut name: Option<String> = None;
        let mut id: Option<String> = None;
        if self.peek(0).kind == TokenKind::String {
            name = self.tokens[self.current].string_value.clone();
            self.current += 1;
        } else {
            while self.peek(0).kind == TokenKind::Identifier && self.peek(1).lexeme == ":" {
                let key = self.identifier("Expected identifier")?.lexeme;
                self.consume(":", "Expected \":\"")?;
                // A value that is not a string ends the arguments, so the closing
                // ')' check below reports it.
                if self.peek(0).kind != TokenKind::String {
                    break;
                }
                let value = self.tokens[self.current].string_value.clone();
                self.current += 1;
                if key == "name" {
                    name = value;
                } else if key == "id" {
                    id = value;
                }
                self.take_if(&[","]);
            }
        }
        self.consume(")", "Expected \")\"")?;
        self.consume("{", "Expected \"{\"")?;
        // The body holds at least one element, so an empty block fails on its
        // missing '.'.
        let mut body = Vec::new();
        loop {
            self.consume(".", "Expected \".\"")?;
            let call = self.parse_call()?;
            if call.name == "write" || call.name == "write3d" {
                return Err(DslError::new(
                    "Expected effect or IO function name",
                    &call.loc,
                ));
            }
            body.push(call);
            if self.check("}") {
                break;
            }
        }
        self.consume("}", "Expected \"}\"")?;
        Ok(DslSubchain {
            name,
            id,
            body,
            loc: start.location(),
        })
    }
    // A chain in a value position (upstream parseChain in its 'expression' context).
    // It may hold subchain blocks, and it cannot hold write() or write3d().
    fn parse_expression_chain(&mut self, first: DslCall) -> Result<DslValue, DslError> {
        let loc = first.loc.clone();
        let mut calls = vec![DslChainElement::Call(first)];
        while self.is_punctuation(".") {
            self.current += 1;
            if self.at_subchain() {
                calls.push(DslChainElement::Subchain(self.parse_subchain()?));
                continue;
            }
            let call = self.parse_call()?;
            // Upstream allows write() and write3d() only in statement context.
            if call.name == "write" || call.name == "write3d" {
                return Err(DslError::new(
                    "Expected effect or IO function name",
                    &call.loc,
                ));
            }
            calls.push(DslChainElement::Call(call));
        }
        Ok(DslValue::Chain(Box::new(DslChain { calls, loc })))
    }
    fn parse_call(&mut self) -> Result<DslCall, DslError> {
        let name = self.identifier("Expected effect or IO function name")?;
        self.consume("(", "Expected \"(\"")?;
        let mut args = Vec::new();
        let mut mode: Option<String> = None;
        // Upstream's parseCall lets the midi() and audio() value calls mix
        // positional and keyword arguments (`allowMixed`); every other call keeps
        // one form.
        let allow_mixed = matches!(name.lexeme.as_str(), "midi" | "audio");
        if !self.check(")") {
            loop {
                let is_named =
                    self.peek(0).kind == TokenKind::Identifier && self.peek(1).lexeme == ":";
                let next_mode = if is_named { "named" } else { "positional" };
                if mode.as_deref().is_some_and(|mode| mode != next_mode) && !allow_mixed {
                    return Err(DslError::new(
                        "Cannot mix positional and named arguments",
                        &self.peek(0).location(),
                    ));
                }
                mode = Some(if mode.as_deref().is_some_and(|mode| mode != next_mode) {
                    "mixed".into()
                } else {
                    next_mode.into()
                });
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
        let call = DslCall {
            name: name.lexeme,
            args,
            arg_mode: mode,
            namespace: None,
            loc,
        };
        Ok(if call.name == "from" {
            self.apply_from_namespace(call)
        } else {
            call
        })
    }

    // Port of upstream transformFromInvocation: `from(namespace, effect(...))` is
    // the effect call, resolved in that one namespace instead of the search order.
    // It takes exactly two positional arguments, an identifier and one call;
    // upstream's read(), read3d(), osc(), midi() and audio() nodes are not Call
    // nodes. Any other from(...) stays a call named `from`, which no namespace
    // defines, so the compiler rejects it as an unknown effect.
    fn apply_from_namespace(&self, call: DslCall) -> DslCall {
        if call.arg_mode.as_deref() != Some("positional") || call.args.len() != 2 {
            return call;
        }
        let [namespace, target] = [call.args[0].value.clone(), call.args[1].value.clone()];
        let inner = match &target {
            DslValue::Chain(chain) if chain.calls.len() == 1 => match &chain.calls[0] {
                DslChainElement::Call(call) => Some(call.clone()),
                DslChainElement::Subchain(_) => None,
            },
            _ => None,
        };
        let namespace_name = match &namespace {
            DslValue::Identifier { name, .. } => Some(name.clone()),
            _ => None,
        };
        if let (Some(namespace), Some(mut inner)) = (namespace_name, inner) {
            if !matches!(inner.name.as_str(), "read" | "read3d" | "write" | "write3d") {
                inner.namespace = Some(namespace);
                return inner;
            }
        }
        call
    }
    fn parse_value_expression(&mut self, minimum: u8) -> Result<DslValue, DslError> {
        let mut left = self.parse_value_unary()?;
        loop {
            if self.peek(0).kind != TokenKind::Operator {
                break;
            }
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
            // `Math.PI` is the one numeric constant of upstream's grammar
            // (`NumberExpr ::= Number | 'Math.PI' | ...`).
            if name == "Math"
                && self.is_punctuation(".")
                && self.peek(1).kind == TokenKind::Identifier
                && self.peek(1).lexeme == "PI"
            {
                self.current += 2;
                return Ok(DslValue::Number(std::f64::consts::PI));
            }
            // A one-argument `read3d(vol0)` names a volume or geometry for a
            // parameter, as upstream's validator reads it. With two arguments it
            // starts a chain.
            if name == "read3d" && self.is_punctuation("(") {
                self.current -= 1;
                let call = self.parse_call()?;
                if call.args.len() == 1
                    && matches!(&call.args[0].value, DslValue::Identifier { .. })
                {
                    return Ok(call.args[0].value.clone());
                }
                return self.parse_expression_chain(call);
            }
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
                // `read(oN).effect()` in a value position is a chain argument that
                // starts from the surface. A bare `read(oN)` stays a plain surface
                // reference.
                if self.is_punctuation(".") {
                    let read = DslCall {
                        name: "read".into(),
                        args: vec![DslArgument {
                            name: None,
                            value: DslValue::Surface(surface.clone()),
                            loc: surface.loc.clone(),
                        }],
                        arg_mode: Some("positional".into()),
                        namespace: None,
                        loc: token.location(),
                    };
                    return self.parse_expression_chain(read);
                }
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
            // `osc(...)`, `midi(...)` and `audio(...)` are the automation value
            // calls of upstream's parser. Oscillators are offline values. Parse
            // MIDI/audio calls only so the compiler can warn and ignore these
            // unsupported real-time inputs.
            if matches!(name.as_str(), "osc" | "midi" | "audio") && self.check("(") {
                self.current -= 1;
                return Ok(DslValue::Call(Box::new(self.parse_call()?)));
            }
            // Any other call in a value position starts a chain argument
            // (upstream `Expr ::= Chain`), for example `tex: solid(color: #f00)`.
            if self.is_punctuation("(") {
                self.current -= 1;
                let call = self.parse_call()?;
                return self.parse_expression_chain(call);
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
