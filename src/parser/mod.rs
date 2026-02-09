pub mod ast;

use crate::lexer::{Token, Span, Spanned};
pub use ast::*;

/// Error de parsing
#[derive(Debug, Clone)]
pub struct ParseError {
    pub message: String,
    pub span: Span,
}

/// Estado del parser
pub struct Parser {
    tokens: Vec<Spanned<Token>>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Spanned<Token>>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn current(&self) -> Option<&Spanned<Token>> {
        self.tokens.get(self.pos)
    }

    fn peek(&self) -> Option<&Token> {
        self.current().map(|t| &t.value)
    }

    fn peek_ahead(&self, n: usize) -> Option<&Token> {
        self.tokens.get(self.pos + n).map(|t| &t.value)
    }

    fn advance(&mut self) -> Option<&Spanned<Token>> {
        let token = self.tokens.get(self.pos);
        self.pos += 1;
        token
    }

    fn skip_newlines(&mut self) {
        while let Some(Token::Newline) = self.peek() {
            self.advance();
        }
    }

    fn expect(&mut self, expected: Token) -> Result<&Spanned<Token>, ParseError> {
        match self.current() {
            Some(t) if std::mem::discriminant(&t.value) == std::mem::discriminant(&expected) => {
                Ok(self.tokens.get(self.pos).unwrap())
            }
            Some(t) => Err(ParseError {
                message: format!("Expected {:?}, found {:?}", expected, t.value),
                span: t.span.clone(),
            }),
            None => Err(ParseError {
                message: format!("Expected {:?}, found end of input", expected),
                span: Span::new(0, 0),
            }),
        }
    }

    fn consume(&mut self, expected: Token) -> Result<Spanned<Token>, ParseError> {
        self.expect(expected)?;
        Ok(self.advance().unwrap().clone())
    }

    fn is_at_end(&self) -> bool {
        self.pos >= self.tokens.len()
    }
}

/// Parse capabilities (+http +json)
fn parse_capabilities(parser: &mut Parser) -> Vec<Capability> {
    let mut caps = Vec::new();

    while let Some(Token::Plus) = parser.peek() {
        let start = parser.current().unwrap().span.start;
        parser.advance(); // consume +

        if let Some(Token::Ident(name)) = parser.peek().cloned() {
            let end = parser.current().unwrap().span.end;
            parser.advance();
            caps.push(Capability {
                name,
                span: Span::new(start, end),
            });
        }

        parser.skip_newlines();
    }

    caps
}

/// Parse a type
fn parse_type(parser: &mut Parser) -> Result<Type, ParseError> {
    let ty = match parser.peek() {
        Some(Token::TypeInt) => { parser.advance(); Type::Int }
        Some(Token::TypeFloat) => { parser.advance(); Type::Float }
        Some(Token::TypeString) => { parser.advance(); Type::String }
        Some(Token::TypeBool) => { parser.advance(); Type::Bool }
        Some(Token::TypeTimestamp) => { parser.advance(); Type::Timestamp }
        Some(Token::TypeUuid) => { parser.advance(); Type::Uuid }
        Some(Token::Ident(name)) => {
            let name = name.clone();
            parser.advance();
            Type::Named(name)
        }
        Some(Token::LBracket) => {
            parser.advance();
            let inner = parse_type(parser)?;
            parser.consume(Token::RBracket)?;
            Type::List(Box::new(inner))
        }
        _ => return Err(ParseError {
            message: "Expected type".to_string(),
            span: parser.current().map(|t| t.span.clone()).unwrap_or(Span::new(0, 0)),
        }),
    };

    // Check for optional marker
    if let Some(Token::Question) = parser.peek() {
        parser.advance();
        Ok(Type::Optional(Box::new(ty)))
    } else {
        Ok(ty)
    }
}

/// Parse an annotation
fn parse_annotation(parser: &mut Parser) -> Result<Option<Annotation>, ParseError> {
    let start = parser.current().map(|t| t.span.start).unwrap_or(0);

    let name = match parser.peek() {
        Some(Token::AnnPk) => { parser.advance(); "pk".to_string() }
        Some(Token::AnnUnique) => { parser.advance(); "unique".to_string() }
        Some(Token::AnnEmail) => { parser.advance(); "email".to_string() }
        Some(Token::AnnUrl) => { parser.advance(); "url".to_string() }
        Some(Token::AnnHash) => { parser.advance(); "hash".to_string() }
        Some(Token::AnnHide) => { parser.advance(); "hide".to_string() }
        Some(Token::AnnAuto) => { parser.advance(); "auto".to_string() }
        Some(Token::AnnRel) => { parser.advance(); "rel".to_string() }
        Some(Token::AnnIndex) => { parser.advance(); "index".to_string() }
        Some(Token::AnnMin) => {
            parser.advance();
            let args = parse_annotation_args(parser)?;
            let end = parser.current().map(|t| t.span.end).unwrap_or(0);
            return Ok(Some(Annotation {
                name: "min".to_string(),
                args,
                span: Span::new(start, end),
            }));
        }
        Some(Token::AnnMax) => {
            parser.advance();
            let args = parse_annotation_args(parser)?;
            let end = parser.current().map(|t| t.span.end).unwrap_or(0);
            return Ok(Some(Annotation {
                name: "max".to_string(),
                args,
                span: Span::new(start, end),
            }));
        }
        _ => return Ok(None),
    };

    let end = parser.tokens.get(parser.pos.saturating_sub(1))
        .map(|t| t.span.end)
        .unwrap_or(0);

    Ok(Some(Annotation {
        name,
        args: vec![],
        span: Span::new(start, end),
    }))
}

fn parse_annotation_args(parser: &mut Parser) -> Result<Vec<Expr>, ParseError> {
    let mut args = Vec::new();

    if let Some(Token::LParen) = parser.peek() {
        parser.advance();

        while parser.peek() != Some(&Token::RParen) && !parser.is_at_end() {
            args.push(parse_expr(parser)?);

            if let Some(Token::Comma) = parser.peek() {
                parser.advance();
            }
        }

        parser.consume(Token::RParen)?;
    }

    Ok(args)
}

/// Parse annotations
fn parse_annotations(parser: &mut Parser) -> Vec<Annotation> {
    let mut anns = Vec::new();

    while let Ok(Some(ann)) = parse_annotation(parser) {
        anns.push(ann);
    }

    anns
}

/// Parse a field in a type definition
fn parse_field(parser: &mut Parser) -> Result<Field, ParseError> {
    let start = parser.current().map(|t| t.span.start).unwrap_or(0);

    let name = match parser.peek() {
        Some(Token::Ident(n)) => {
            let n = n.clone();
            parser.advance();
            n
        }
        _ => return Err(ParseError {
            message: "Expected field name".to_string(),
            span: parser.current().map(|t| t.span.clone()).unwrap_or(Span::new(0, 0)),
        }),
    };

    let ty = parse_type(parser)?;

    // Check for nullable (already handled in parse_type for Type::Optional)
    let nullable = matches!(ty, Type::Optional(_));

    let annotations = parse_annotations(parser);

    let end = parser.tokens.get(parser.pos.saturating_sub(1))
        .map(|t| t.span.end)
        .unwrap_or(0);

    Ok(Field {
        name,
        ty,
        nullable,
        default: None,
        annotations,
        span: Span::new(start, end),
    })
}

/// Parse a type definition
fn parse_type_def(parser: &mut Parser) -> Result<TypeDef, ParseError> {
    let start = parser.current().map(|t| t.span.start).unwrap_or(0);

    parser.consume(Token::At)?;

    let name = match parser.peek() {
        Some(Token::Ident(n)) => {
            let n = n.clone();
            parser.advance();
            n
        }
        _ => return Err(ParseError {
            message: "Expected type name".to_string(),
            span: parser.current().map(|t| t.span.clone()).unwrap_or(Span::new(0, 0)),
        }),
    };

    parser.skip_newlines();
    parser.consume(Token::LBrace)?;
    parser.skip_newlines();

    let mut fields = Vec::new();

    while parser.peek() != Some(&Token::RBrace) && !parser.is_at_end() {
        fields.push(parse_field(parser)?);
        parser.skip_newlines();
    }

    parser.consume(Token::RBrace)?;

    let annotations = parse_annotations(parser);

    let end = parser.tokens.get(parser.pos.saturating_sub(1))
        .map(|t| t.span.end)
        .unwrap_or(0);

    Ok(TypeDef {
        name,
        fields,
        annotations,
        span: Span::new(start, end),
    })
}

/// Parse an expression
fn parse_expr(parser: &mut Parser) -> Result<Expr, ParseError> {
    parse_pipe(parser)
}

fn parse_pipe(parser: &mut Parser) -> Result<Expr, ParseError> {
    let mut left = parse_comparison(parser)?;

    while let Some(Token::PipeOp) = parser.peek() {
        parser.advance();
        let right = parse_comparison(parser)?;

        match left {
            Expr::Pipe(ref mut exprs) => exprs.push(right),
            _ => left = Expr::Pipe(vec![left, right]),
        }
    }

    Ok(left)
}

fn parse_comparison(parser: &mut Parser) -> Result<Expr, ParseError> {
    let mut left = parse_additive(parser)?;

    while let Some(op) = match parser.peek() {
        Some(Token::EqEq) => Some(BinaryOp::Eq),
        Some(Token::NotEq) => Some(BinaryOp::NotEq),
        Some(Token::Lt) => Some(BinaryOp::Lt),
        Some(Token::Gt) => Some(BinaryOp::Gt),
        Some(Token::LtEq) => Some(BinaryOp::LtEq),
        Some(Token::GtEq) => Some(BinaryOp::GtEq),
        _ => None,
    } {
        parser.advance();
        let right = parse_additive(parser)?;
        left = Expr::BinaryOp {
            left: Box::new(left),
            op,
            right: Box::new(right),
        };
    }

    Ok(left)
}

fn parse_additive(parser: &mut Parser) -> Result<Expr, ParseError> {
    let mut left = parse_multiplicative(parser)?;

    while let Some(op) = match parser.peek() {
        Some(Token::Plus) => Some(BinaryOp::Add),
        Some(Token::Minus) => Some(BinaryOp::Sub),
        Some(Token::PlusPlus) => Some(BinaryOp::Concat),
        _ => None,
    } {
        parser.advance();
        let right = parse_multiplicative(parser)?;
        left = Expr::BinaryOp {
            left: Box::new(left),
            op,
            right: Box::new(right),
        };
    }

    Ok(left)
}

fn parse_multiplicative(parser: &mut Parser) -> Result<Expr, ParseError> {
    let mut left = parse_unary(parser)?;

    while let Some(op) = match parser.peek() {
        Some(Token::Star) => Some(BinaryOp::Mul),
        Some(Token::Slash) => Some(BinaryOp::Div),
        Some(Token::Percent) => Some(BinaryOp::Mod),
        _ => None,
    } {
        parser.advance();
        let right = parse_unary(parser)?;
        left = Expr::BinaryOp {
            left: Box::new(left),
            op,
            right: Box::new(right),
        };
    }

    Ok(left)
}

fn parse_unary(parser: &mut Parser) -> Result<Expr, ParseError> {
    match parser.peek() {
        Some(Token::Minus) => {
            parser.advance();
            let expr = parse_unary(parser)?;
            Ok(Expr::UnaryOp {
                op: UnaryOp::Neg,
                expr: Box::new(expr),
            })
        }
        Some(Token::Bang) => {
            parser.advance();
            let expr = parse_unary(parser)?;
            Ok(Expr::UnaryOp {
                op: UnaryOp::Not,
                expr: Box::new(expr),
            })
        }
        _ => parse_call(parser),
    }
}

/// Determina si una expresión puede ser llamada como función
fn is_callable(expr: &Expr) -> bool {
    matches!(expr,
        Expr::Ident(_) |
        Expr::FieldAccess(_, _) |
        Expr::SafeAccess(_, _) |
        Expr::Call { .. } |
        Expr::Lambda { .. }
    )
}

fn parse_call(parser: &mut Parser) -> Result<Expr, ParseError> {
    let mut expr = parse_primary(parser)?;

    loop {
        match parser.peek() {
            Some(Token::LParen) => {
                // Solo permitir llamadas en expresiones que pueden ser funciones
                // (identificadores, accesos a campos, otras llamadas, lambdas)
                // No en literales (Int, Float, String, Bool, etc.)
                if !is_callable(&expr) {
                    break;
                }

                parser.advance();
                let mut args = Vec::new();

                while parser.peek() != Some(&Token::RParen) && !parser.is_at_end() {
                    args.push(parse_expr(parser)?);

                    if let Some(Token::Comma) = parser.peek() {
                        parser.advance();
                    } else if parser.peek() != Some(&Token::RParen) {
                        // Allow space-separated args
                    }
                }

                parser.consume(Token::RParen)?;

                expr = Expr::Call {
                    func: Box::new(expr),
                    args,
                    has_effect: false,
                };
            }
            Some(Token::Bang) => {
                // Check if it's a call with effect: func!(args)
                if parser.peek_ahead(1) == Some(&Token::LParen) && is_callable(&expr) {
                    parser.advance(); // consume !
                    parser.advance(); // consume (

                    let mut args = Vec::new();

                    while parser.peek() != Some(&Token::RParen) && !parser.is_at_end() {
                        args.push(parse_expr(parser)?);

                        if let Some(Token::Comma) = parser.peek() {
                            parser.advance();
                        }
                    }

                    parser.consume(Token::RParen)?;

                    expr = Expr::Call {
                        func: Box::new(expr),
                        args,
                        has_effect: true,
                    };
                } else {
                    break;
                }
            }
            Some(Token::Dot) => {
                parser.advance();

                if let Some(Token::Ident(field)) = parser.peek().cloned() {
                    parser.advance();
                    expr = Expr::FieldAccess(Box::new(expr), field);
                } else {
                    return Err(ParseError {
                        message: "Expected field name after '.'".to_string(),
                        span: parser.current().map(|t| t.span.clone()).unwrap_or(Span::new(0, 0)),
                    });
                }
            }
            Some(Token::SafeNav) => {
                parser.advance();

                if let Some(Token::Ident(field)) = parser.peek().cloned() {
                    parser.advance();
                    expr = Expr::SafeAccess(Box::new(expr), field);
                } else {
                    return Err(ParseError {
                        message: "Expected field name after '?.'".to_string(),
                        span: parser.current().map(|t| t.span.clone()).unwrap_or(Span::new(0, 0)),
                    });
                }
            }
            _ => break,
        }
    }

    Ok(expr)
}

fn parse_primary(parser: &mut Parser) -> Result<Expr, ParseError> {
    match parser.peek().cloned() {
        Some(Token::Int(n)) => {
            parser.advance();
            Ok(Expr::Int(n))
        }
        Some(Token::Float(n)) => {
            parser.advance();
            Ok(Expr::Float(n))
        }
        Some(Token::String(s)) => {
            parser.advance();
            Ok(Expr::String(s))
        }
        Some(Token::True) => {
            parser.advance();
            Ok(Expr::Bool(true))
        }
        Some(Token::False) => {
            parser.advance();
            Ok(Expr::Bool(false))
        }
        Some(Token::Nil) => {
            parser.advance();
            Ok(Expr::Nil)
        }
        Some(Token::Underscore) => {
            parser.advance();
            Ok(Expr::Placeholder)
        }
        Some(Token::Ident(name)) => {
            parser.advance();
            Ok(Expr::Ident(name))
        }
        Some(Token::LParen) => {
            parser.advance();
            let expr = parse_expr(parser)?;
            parser.consume(Token::RParen)?;
            Ok(expr)
        }
        Some(Token::LBracket) => {
            parser.advance();
            let mut items = Vec::new();

            while parser.peek() != Some(&Token::RBracket) && !parser.is_at_end() {
                items.push(parse_expr(parser)?);

                if let Some(Token::Comma) = parser.peek() {
                    parser.advance();
                }
            }

            parser.consume(Token::RBracket)?;
            Ok(Expr::List(items))
        }
        Some(Token::LBrace) => {
            parser.advance();
            let mut fields = Vec::new();

            while parser.peek() != Some(&Token::RBrace) && !parser.is_at_end() {
                if let Some(Token::Ident(name)) = parser.peek().cloned() {
                    parser.advance();
                    parser.consume(Token::Colon)?;
                    let value = parse_expr(parser)?;
                    fields.push((name, value));

                    if let Some(Token::Comma) = parser.peek() {
                        parser.advance();
                    }
                } else {
                    break;
                }
            }

            parser.consume(Token::RBrace)?;
            Ok(Expr::Record(fields))
        }
        Some(Token::Colon) => {
            // Block expression: : expr1; expr2; expr3
            parser.advance();
            parse_block(parser)
        }
        Some(Token::If) => {
            // If expression: if cond then_expr else else_expr
            parser.advance();
            let condition = parse_expr(parser)?;
            let then_branch = parse_expr(parser)?;

            let else_branch = if matches!(parser.peek(), Some(Token::Else)) {
                parser.advance();
                Some(Box::new(parse_expr(parser)?))
            } else {
                None
            };

            Ok(Expr::If {
                condition: Box::new(condition),
                then_branch: Box::new(then_branch),
                else_branch,
            })
        }
        Some(Token::Question) => {
            // Match expression: ? cond -> expr | cond -> expr | _ -> expr
            parser.advance();
            parse_match_expr(parser)
        }
        Some(Token::Minus) => {
            // Unary minus: -expr
            parser.advance();
            let expr = parse_primary(parser)?;
            Ok(Expr::UnaryOp {
                op: UnaryOp::Neg,
                expr: Box::new(expr),
            })
        }
        Some(Token::Bang) => {
            // Unary not: !expr
            parser.advance();
            let expr = parse_primary(parser)?;
            Ok(Expr::UnaryOp {
                op: UnaryOp::Not,
                expr: Box::new(expr),
            })
        }
        Some(Token::Expect) => {
            // Expect expression: expect condition "optional message"
            parser.advance();
            let condition = parse_comparison(parser)?;

            // Check for optional message (string literal)
            let message = if let Some(Token::String(msg)) = parser.peek().cloned() {
                parser.advance();
                Some(msg)
            } else {
                None
            };

            Ok(Expr::Expect {
                condition: Box::new(condition),
                message,
            })
        }
        Some(Token::Observe) => {
            parse_observe_expr(parser)
        }
        Some(Token::Reason) => {
            parse_reason_expr(parser)
        }
        _ => Err(ParseError {
            message: format!("Unexpected token: {:?}", parser.peek()),
            span: parser.current().map(|t| t.span.clone()).unwrap_or(Span::new(0, 0)),
        }),
    }
}

/// Parse a block expression (: expr1; expr2; exprN)
/// Each `name = expr` inside becomes a Let binding
/// The last expression is the return value
/// Block ends at newline or EOF
fn parse_block(parser: &mut Parser) -> Result<Expr, ParseError> {
    let mut exprs = Vec::new();

    // Skip leading whitespace (but not newlines - those end the block on same line)
    while matches!(parser.peek(), Some(Token::Semicolon)) {
        parser.advance();
    }

    while !parser.is_at_end() {
        // Newline ends the block (return to top-level parsing)
        if matches!(parser.peek(), Some(Token::Newline)) {
            break;
        }

        // Try to parse a let binding (name = expr) or regular expression
        let expr = parse_block_item(parser)?;
        exprs.push(expr);

        // Semicolon separates expressions in the block
        if matches!(parser.peek(), Some(Token::Semicolon)) {
            parser.advance();
            // Skip additional semicolons
            while matches!(parser.peek(), Some(Token::Semicolon)) {
                parser.advance();
            }
        } else {
            // No semicolon - block ends (newline, EOF, or other token)
            break;
        }
    }

    if exprs.is_empty() {
        Ok(Expr::Nil)
    } else {
        Ok(Expr::Block(exprs))
    }
}


/// Parse a single item in a block (either a let binding or expression)
fn parse_block_item(parser: &mut Parser) -> Result<Expr, ParseError> {
    // Check if this looks like a let binding: Ident = expr (but NOT Ident() = or Ident(x) =)
    if let Some(Token::Ident(name)) = parser.peek().cloned() {
        if parser.peek_ahead(1) == Some(&Token::Eq) {
            // Make sure it's not a function definition (no parens before =)
            parser.advance(); // consume ident
            parser.advance(); // consume =
            let value = parse_expr(parser)?;
            return Ok(Expr::Let {
                name,
                value: Box::new(value),
            });
        }
    }

    // Otherwise parse as a regular expression
    parse_expr(parser)
}

/// Parse a match/conditional expression: ? cond -> expr | cond -> expr | _ -> default
fn parse_match_expr(parser: &mut Parser) -> Result<Expr, ParseError> {
    let mut arms = Vec::new();

    loop {
        // Skip newlines between arms
        parser.skip_newlines();

        // Check for wildcard pattern
        let pattern = if matches!(parser.peek(), Some(Token::Underscore)) {
            parser.advance();
            Pattern::Wildcard
        } else {
            // Parse condition expression as pattern
            let expr = parse_comparison(parser)?;
            Pattern::Literal(expr)
        };

        // Expect ->
        if !matches!(parser.peek(), Some(Token::Arrow)) {
            return Err(ParseError {
                message: format!("Expected '->' in match arm, found {:?}", parser.peek()),
                span: parser.current().map(|t| t.span.clone()).unwrap_or(Span::new(0, 0)),
            });
        }
        parser.advance();

        // Parse body expression
        let body = parse_comparison(parser)?;

        arms.push(MatchArm { pattern, body });

        // Check for more arms (|)
        if matches!(parser.peek(), Some(Token::Pipe)) {
            parser.advance();
        } else {
            break;
        }
    }

    // Create match expression with dummy expression (the condition is in the patterns)
    // For now, we'll evaluate patterns as boolean conditions
    if arms.is_empty() {
        return Err(ParseError {
            message: "Match expression requires at least one arm".to_string(),
            span: parser.current().map(|t| t.span.clone()).unwrap_or(Span::new(0, 0)),
        });
    }

    // Convert to If chain for simple conditional matching
    // ? cond1 -> expr1 | cond2 -> expr2 | _ -> default
    // becomes: if cond1 then expr1 else if cond2 then expr2 else default
    let mut result = None;

    for arm in arms.into_iter().rev() {
        let body_expr = arm.body;
        match arm.pattern {
            Pattern::Wildcard => {
                result = Some(body_expr);
            }
            Pattern::Literal(cond) => {
                let else_branch = result.map(Box::new);
                result = Some(Expr::If {
                    condition: Box::new(cond),
                    then_branch: Box::new(body_expr),
                    else_branch,
                });
            }
            _ => {
                // Other patterns not yet supported, treat as condition
                result = Some(body_expr);
            }
        }
    }

    result.ok_or_else(|| ParseError {
        message: "Empty match expression".to_string(),
        span: parser.current().map(|t| t.span.clone()).unwrap_or(Span::new(0, 0)),
    })
}

/// Parse @self_heal annotation with optional parameters
/// @self_heal or @self_heal(max_attempts: 3, mode: "technical")
fn parse_self_heal_config(parser: &mut Parser) -> Result<SelfHealConfig, ParseError> {
    let mut config = SelfHealConfig::default();

    // Check for optional parameters
    if let Some(Token::LParen) = parser.peek() {
        parser.advance(); // consume (

        while parser.peek() != Some(&Token::RParen) && !parser.is_at_end() {
            // Parse key: value pairs
            if let Some(Token::Ident(key)) = parser.peek().cloned() {
                parser.advance();
                parser.consume(Token::Colon)?;

                match key.as_str() {
                    "max_attempts" => {
                        if let Some(Token::Int(n)) = parser.peek() {
                            config.max_attempts = *n as u32;
                            parser.advance();
                        }
                    }
                    "mode" => {
                        if let Some(Token::String(s)) = parser.peek().cloned() {
                            config.mode = HealMode::from_str(&s);
                            parser.advance();
                        }
                    }
                    _ => {
                        // Skip unknown parameter value
                        parser.advance();
                    }
                }

                // Skip comma between params
                if let Some(Token::Comma) = parser.peek() {
                    parser.advance();
                }
            } else {
                break;
            }
        }

        parser.consume(Token::RParen)?;
    }

    Ok(config)
}

/// Parse a function definition, optionally preceded by @self_heal
fn parse_func_def(parser: &mut Parser) -> Result<FuncDef, ParseError> {
    parse_func_def_with_self_heal(parser, None)
}

/// Parse a function definition with an optional pre-parsed self_heal config
fn parse_func_def_with_self_heal(parser: &mut Parser, self_heal: Option<SelfHealConfig>) -> Result<FuncDef, ParseError> {
    let start = parser.current().map(|t| t.span.start).unwrap_or(0);

    let name = match parser.peek() {
        Some(Token::Ident(n)) => {
            let n = n.clone();
            parser.advance();
            n
        }
        _ => return Err(ParseError {
            message: "Expected function name".to_string(),
            span: parser.current().map(|t| t.span.clone()).unwrap_or(Span::new(0, 0)),
        }),
    };

    // Check for effect marker
    let has_effect = if let Some(Token::Bang) = parser.peek() {
        parser.advance();
        true
    } else {
        false
    };

    // Parameters are optional: name(params) = expr OR name = expr
    let mut params = Vec::new();

    if let Some(Token::LParen) = parser.peek() {
        parser.advance(); // consume (

        while parser.peek() != Some(&Token::RParen) && !parser.is_at_end() {
            if let Some(Token::Ident(param_name)) = parser.peek().cloned() {
                parser.advance();
                params.push(Param {
                    name: param_name,
                    ty: None,
                });

                if let Some(Token::Comma) = parser.peek() {
                    parser.advance();
                }
            } else {
                break;
            }
        }

        parser.consume(Token::RParen)?;
    }

    parser.consume(Token::Eq)?;

    let body = parse_expr(parser)?;

    let end = parser.tokens.get(parser.pos.saturating_sub(1))
        .map(|t| t.span.end)
        .unwrap_or(0);

    Ok(FuncDef {
        name,
        has_effect,
        params,
        return_type: None,
        body,
        span: Span::new(start, end),
        self_heal,
    })
}

/// Parse a goal declaration: goal "description" [check <expr>]
fn parse_goal(parser: &mut Parser) -> Result<GoalDef, ParseError> {
    let start = parser.current().map(|t| t.span.start).unwrap_or(0);
    parser.consume(Token::Goal)?;

    let description = match parser.peek().cloned() {
        Some(Token::String(s)) => {
            parser.advance();
            s
        }
        _ => return Err(ParseError {
            message: "Expected string after 'goal'".to_string(),
            span: parser.current().map(|t| t.span.clone()).unwrap_or(Span::new(0, 0)),
        }),
    };

    // Check for optional "check" keyword (soft keyword via Ident)
    let check = if let Some(Token::Ident(ref s)) = parser.peek().cloned() {
        if s == "check" {
            parser.advance(); // consume "check"
            Some(parse_expr(parser)?)
        } else {
            None
        }
    } else {
        None
    };

    let end = parser.tokens.get(parser.pos.saturating_sub(1))
        .map(|t| t.span.end)
        .unwrap_or(0);

    Ok(GoalDef {
        description,
        check,
        span: Span::new(start, end),
    })
}

/// Parse an observe declaration: observe <ident> [where <expr>]
fn parse_observe_expr(parser: &mut Parser) -> Result<Expr, ParseError> {
    parser.consume(Token::Observe)?;

    let target = match parser.peek().cloned() {
        Some(Token::Ident(name)) => {
            parser.advance();
            name
        }
        _ => return Err(ParseError {
            message: "Expected identifier after 'observe'".to_string(),
            span: parser.current().map(|t| t.span.clone()).unwrap_or(Span::new(0, 0)),
        }),
    };

    // Handle dotted access: observe response.status
    let mut full_target = target;
    while let Some(Token::Dot) = parser.peek() {
        parser.advance();
        if let Some(Token::Ident(field)) = parser.peek().cloned() {
            parser.advance();
            full_target = format!("{}.{}", full_target, field);
        }
    }

    // Check for optional "where" condition
    let condition = if let Some(Token::Where) = parser.peek() {
        parser.advance();
        Some(Box::new(parse_expr(parser)?))
    } else {
        None
    };

    Ok(Expr::Observe {
        target: full_target,
        condition,
    })
}

/// Parse an observe definition at top level
fn parse_observe_def(parser: &mut Parser) -> Result<ObserveDef, ParseError> {
    let start = parser.current().map(|t| t.span.start).unwrap_or(0);
    parser.consume(Token::Observe)?;

    let target = match parser.peek().cloned() {
        Some(Token::Ident(name)) => {
            parser.advance();
            name
        }
        _ => return Err(ParseError {
            message: "Expected identifier after 'observe'".to_string(),
            span: parser.current().map(|t| t.span.clone()).unwrap_or(Span::new(0, 0)),
        }),
    };

    // Handle dotted access
    let mut full_target = target;
    while let Some(Token::Dot) = parser.peek() {
        parser.advance();
        if let Some(Token::Ident(field)) = parser.peek().cloned() {
            parser.advance();
            full_target = format!("{}.{}", full_target, field);
        }
    }

    let condition = if let Some(Token::Where) = parser.peek() {
        parser.advance();
        Some(parse_expr(parser)?)
    } else {
        None
    };

    let end = parser.tokens.get(parser.pos.saturating_sub(1))
        .map(|t| t.span.end)
        .unwrap_or(0);

    Ok(ObserveDef {
        target: full_target,
        condition,
        span: Span::new(start, end),
    })
}

/// Parse a reason expression
/// Simple: reason "question"
/// Structured: reason { observed: [expr, ...], question: "question" }
fn parse_reason_expr(parser: &mut Parser) -> Result<Expr, ParseError> {
    parser.consume(Token::Reason)?;

    match parser.peek().cloned() {
        Some(Token::String(question)) => {
            parser.advance();
            Ok(Expr::Reason {
                observations: Vec::new(),
                question,
            })
        }
        Some(Token::LBrace) => {
            parser.advance(); // consume {
            let mut observations = Vec::new();
            let mut question = String::new();

            while parser.peek() != Some(&Token::RBrace) && !parser.is_at_end() {
                parser.skip_newlines();

                if let Some(Token::Ident(key)) = parser.peek().cloned() {
                    parser.advance();
                    parser.consume(Token::Colon)?;

                    match key.as_str() {
                        "observed" => {
                            parser.consume(Token::LBracket)?;
                            while parser.peek() != Some(&Token::RBracket) && !parser.is_at_end() {
                                observations.push(parse_expr(parser)?);
                                if let Some(Token::Comma) = parser.peek() {
                                    parser.advance();
                                }
                            }
                            parser.consume(Token::RBracket)?;
                        }
                        "question" => {
                            if let Some(Token::String(q)) = parser.peek().cloned() {
                                parser.advance();
                                question = q;
                            }
                        }
                        _ => {
                            // Skip unknown key's value
                            parser.advance();
                        }
                    }

                    // Skip comma between fields
                    if let Some(Token::Comma) = parser.peek() {
                        parser.advance();
                    }
                    parser.skip_newlines();
                } else {
                    break;
                }
            }

            parser.consume(Token::RBrace)?;

            Ok(Expr::Reason {
                observations,
                question,
            })
        }
        _ => Err(ParseError {
            message: "Expected string or { after 'reason'".to_string(),
            span: parser.current().map(|t| t.span.clone()).unwrap_or(Span::new(0, 0)),
        }),
    }
}

/// Parse an invariant declaration: invariant <expression>
/// Invariants are constraints that the healing system cannot violate
fn parse_invariant(parser: &mut Parser) -> Result<Expr, ParseError> {
    parser.consume(Token::Invariant)?;
    parse_expr(parser)
}

/// Parse a definition (type, function, goal, invariant, observe, or annotated function)
fn parse_definition(parser: &mut Parser) -> Result<Option<Definition>, ParseError> {
    parser.skip_newlines();

    match parser.peek() {
        Some(Token::Goal) => {
            Ok(Some(Definition::Goal(parse_goal(parser)?)))
        }
        Some(Token::Invariant) => {
            Ok(Some(Definition::Invariant(parse_invariant(parser)?)))
        }
        Some(Token::Observe) => {
            Ok(Some(Definition::Observe(parse_observe_def(parser)?)))
        }
        Some(Token::AnnSelfHeal) => {
            // @self_heal annotation followed by function definition
            parser.advance(); // consume @self_heal
            let config = parse_self_heal_config(parser)?;
            parser.skip_newlines();

            // Now expect a function definition
            if let Some(Token::Ident(_)) = parser.peek() {
                Ok(Some(Definition::FuncDef(parse_func_def_with_self_heal(parser, Some(config))?)))
            } else {
                Err(ParseError {
                    message: "Expected function definition after @self_heal".to_string(),
                    span: parser.current().map(|t| t.span.clone()).unwrap_or(Span::new(0, 0)),
                })
            }
        }
        Some(Token::At) => {
            Ok(Some(Definition::TypeDef(parse_type_def(parser)?)))
        }
        Some(Token::Ident(_)) => {
            Ok(Some(Definition::FuncDef(parse_func_def(parser)?)))
        }
        None => Ok(None),
        _ => {
            parser.advance(); // Skip unknown token
            Ok(None)
        }
    }
}

/// Parse a complete program
pub fn parse(tokens: Vec<Spanned<Token>>) -> Result<Program, Vec<ParseError>> {
    let mut parser = Parser::new(tokens);
    let mut errors = Vec::new();

    parser.skip_newlines();

    let capabilities = parse_capabilities(&mut parser);

    let mut definitions = Vec::new();

    while !parser.is_at_end() {
        match parse_definition(&mut parser) {
            Ok(Some(def)) => definitions.push(def),
            Ok(None) => {}
            Err(e) => {
                errors.push(e);
                parser.advance(); // Skip problematic token
            }
        }
        parser.skip_newlines();
    }

    if errors.is_empty() {
        Ok(Program {
            capabilities,
            definitions,
        })
    } else {
        Err(errors)
    }
}

/// Parsea y retorna JSON
pub fn parse_json(tokens: Vec<Spanned<Token>>) -> String {
    match parse(tokens) {
        Ok(program) => serde_json::to_string_pretty(&program).unwrap_or_default(),
        Err(errors) => serde_json::to_string_pretty(
            &errors.iter().map(|e| &e.message).collect::<Vec<_>>()
        ).unwrap_or_default(),
    }
}

/// Parsea una sola expresion (para REPL)
pub fn parse_expression(tokens: Vec<Spanned<Token>>) -> Result<Expr, ParseError> {
    let mut parser = Parser::new(tokens);
    parser.skip_newlines();
    parse_expr(&mut parser)
}

/// Parsea una expresion completa, fallando si quedan tokens sin consumir
/// Usado para interpolación de strings donde queremos asegurar parsing completo
pub fn parse_expression_complete(tokens: Vec<Spanned<Token>>) -> Result<Expr, ParseError> {
    let mut parser = Parser::new(tokens);
    parser.skip_newlines();
    let expr = parse_expr(&mut parser)?;

    // Saltar newlines finales
    parser.skip_newlines();

    // Verificar que no quedan tokens
    if !parser.is_at_end() {
        let remaining = parser.current().map(|t| format!("{:?}", t.value))
            .unwrap_or_else(|| "unknown".to_string());
        return Err(ParseError {
            message: format!("Tokens no consumidos después de la expresión: {}", remaining),
            span: parser.current().map(|t| t.span.clone()).unwrap_or(Span { start: 0, end: 0 }),
        });
    }

    Ok(expr)
}

/// Parsea una definicion de funcion (para REPL)
pub fn parse_function_def(tokens: Vec<Spanned<Token>>) -> Result<FuncDef, ParseError> {
    let mut parser = Parser::new(tokens);
    parser.skip_newlines();
    parse_func_def(&mut parser)
}

/// Determina si el input parece ser una definicion de funcion
/// Una definicion tiene la forma: nombre(params) = expr o nombre = expr
/// Tambien soporta @self_heal antes de la definicion
pub fn looks_like_function_def(tokens: &[Spanned<Token>]) -> bool {
    // Buscar patron: Ident seguido de ( o =
    let mut i = 0;

    // Saltar newlines
    while i < tokens.len() && matches!(tokens[i].value, Token::Newline) {
        i += 1;
    }

    // Check for @self_heal annotation
    if i < tokens.len() && matches!(tokens[i].value, Token::AnnSelfHeal) {
        i += 1;
        // Skip optional parameters @self_heal(...)
        if i < tokens.len() && matches!(tokens[i].value, Token::LParen) {
            let mut paren_depth = 1;
            i += 1;
            while i < tokens.len() && paren_depth > 0 {
                match &tokens[i].value {
                    Token::LParen => paren_depth += 1,
                    Token::RParen => paren_depth -= 1,
                    _ => {}
                }
                i += 1;
            }
        }
        // Skip newlines after annotation
        while i < tokens.len() && matches!(tokens[i].value, Token::Newline) {
            i += 1;
        }
    }

    // Debe empezar con identificador
    if i >= tokens.len() || !matches!(tokens[i].value, Token::Ident(_)) {
        return false;
    }
    i += 1;

    // Siguiente puede ser ! (effect), ( o =
    if i >= tokens.len() {
        return false;
    }

    // Si tiene !, avanzar
    if matches!(tokens[i].value, Token::Bang) {
        i += 1;
        if i >= tokens.len() {
            return false;
        }
    }

    // Si tiene (, buscar ) y luego =
    if matches!(tokens[i].value, Token::LParen) {
        // Buscar el cierre de parentesis
        let mut paren_depth = 1;
        i += 1;
        while i < tokens.len() && paren_depth > 0 {
            match &tokens[i].value {
                Token::LParen => paren_depth += 1,
                Token::RParen => paren_depth -= 1,
                _ => {}
            }
            i += 1;
        }
        // Despues del ) debe venir =
        if i < tokens.len() && matches!(tokens[i].value, Token::Eq) {
            return true;
        }
    }

    // Si es directamente =, es una definicion sin parametros (constante/funcion sin args)
    if matches!(tokens[i].value, Token::Eq) {
        return true;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::tokenize;

    #[test]
    fn test_parse_capabilities() {
        let tokens = tokenize("+http +json\n").unwrap();
        let program = parse(tokens).unwrap();
        assert_eq!(program.capabilities.len(), 2);
        assert_eq!(program.capabilities[0].name, "http");
        assert_eq!(program.capabilities[1].name, "json");
    }

    #[test]
    fn test_parse_simple_function() {
        let tokens = tokenize("+http\nadd(a b) = a + b\n").unwrap();
        let program = parse(tokens).unwrap();
        assert_eq!(program.definitions.len(), 1);
        if let Definition::FuncDef(f) = &program.definitions[0] {
            assert_eq!(f.name, "add");
            assert_eq!(f.params.len(), 2);
        } else {
            panic!("Expected function definition");
        }
    }

    #[test]
    fn test_parse_function_with_effect() {
        let tokens = tokenize("+http\nfetch!(url) = http.get(url)\n").unwrap();
        let program = parse(tokens).unwrap();
        if let Definition::FuncDef(f) = &program.definitions[0] {
            assert_eq!(f.name, "fetch");
            assert!(f.has_effect);
        } else {
            panic!("Expected function definition");
        }
    }

    #[test]
    fn test_parse_type_def() {
        let tokens = tokenize("+http\n@User {\nid:uuid @pk\nname:s\n}\n").unwrap();
        let program = parse(tokens).unwrap();
        if let Definition::TypeDef(t) = &program.definitions[0] {
            assert_eq!(t.name, "User");
            assert_eq!(t.fields.len(), 2);
            assert_eq!(t.fields[0].name, "id");
            assert_eq!(t.fields[1].name, "name");
        } else {
            panic!("Expected type definition");
        }
    }

    #[test]
    fn test_parse_complete_example() {
        let source = r#"+http +json

@User {
    id:uuid @pk
    name:s @min(2)
    email:s? @email
}

greeting(name) = "Hello {name}!"

fetch(id) = http.get("users/{id}").json(User)
"#;
        let tokens = tokenize(source).unwrap();
        let program = parse(tokens).unwrap();

        assert_eq!(program.capabilities.len(), 2);
        assert_eq!(program.definitions.len(), 3); // 1 type + 2 functions
    }

    #[test]
    fn test_parse_goal() {
        let source = r#"+http
goal "fetch user and return profile"
main = 42
"#;
        let tokens = tokenize(source).unwrap();
        let program = parse(tokens).unwrap();

        assert_eq!(program.definitions.len(), 2); // 1 goal + 1 function
        if let Definition::Goal(g) = &program.definitions[0] {
            assert_eq!(g.description, "fetch user and return profile");
            assert!(g.check.is_none());
        } else {
            panic!("Expected goal definition");
        }
    }

    #[test]
    fn test_parse_multiple_goals() {
        let source = r#"+http
goal "primary goal"
goal "secondary goal"
main = 42
"#;
        let tokens = tokenize(source).unwrap();
        let program = parse(tokens).unwrap();

        assert_eq!(program.definitions.len(), 3); // 2 goals + 1 function
        if let Definition::Goal(g1) = &program.definitions[0] {
            assert_eq!(g1.description, "primary goal");
        } else {
            panic!("Expected first goal");
        }
        if let Definition::Goal(g2) = &program.definitions[1] {
            assert_eq!(g2.description, "secondary goal");
        } else {
            panic!("Expected second goal");
        }
    }

    #[test]
    fn test_parse_goal_with_check() {
        let source = r#"+http
goal "all values positive" check x > 0
main = 42
"#;
        let tokens = tokenize(source).unwrap();
        let program = parse(tokens).unwrap();

        if let Definition::Goal(g) = &program.definitions[0] {
            assert_eq!(g.description, "all values positive");
            assert!(g.check.is_some());
        } else {
            panic!("Expected goal definition");
        }
    }

    #[test]
    fn test_parse_observe() {
        let source = r#"+http
observe users
main = 42
"#;
        let tokens = tokenize(source).unwrap();
        let program = parse(tokens).unwrap();

        assert!(matches!(&program.definitions[0], Definition::Observe(_)));
    }

    #[test]
    fn test_parse_observe_in_block() {
        let source = r#"+http
main = : observe x; x
"#;
        let tokens = tokenize(source).unwrap();
        let program = parse(tokens).unwrap();
        // Should parse without error
        assert!(!program.definitions.is_empty());
    }

    #[test]
    fn test_parse_reason_simple() {
        let source = r#"+http
main = : result = reason "should I retry?"; result
"#;
        let tokens = tokenize(source).unwrap();
        let program = parse(tokens).unwrap();
        assert!(!program.definitions.is_empty());
    }

    #[test]
    fn test_parse_self_heal_simple() {
        let source = r#"+http
@self_heal
main = 42
"#;
        let tokens = tokenize(source).unwrap();
        let program = parse(tokens).unwrap();

        assert_eq!(program.definitions.len(), 1);
        if let Definition::FuncDef(f) = &program.definitions[0] {
            assert_eq!(f.name, "main");
            assert!(f.self_heal.is_some());
            let config = f.self_heal.as_ref().unwrap();
            assert_eq!(config.max_attempts, 3); // default
            assert_eq!(config.mode, HealMode::Auto); // default
        } else {
            panic!("Expected function definition");
        }
    }

    #[test]
    fn test_parse_self_heal_with_params() {
        let source = r#"+http
@self_heal(max_attempts: 5, mode: "technical")
risky_op() = http.get("api/data")
"#;
        let tokens = tokenize(source).unwrap();
        let program = parse(tokens).unwrap();

        assert_eq!(program.definitions.len(), 1);
        if let Definition::FuncDef(f) = &program.definitions[0] {
            assert_eq!(f.name, "risky_op");
            assert!(f.self_heal.is_some());
            let config = f.self_heal.as_ref().unwrap();
            assert_eq!(config.max_attempts, 5);
            assert_eq!(config.mode, HealMode::Technical);
        } else {
            panic!("Expected function definition");
        }
    }

    #[test]
    fn test_parse_self_heal_semantic_mode() {
        let source = r#"+http
@self_heal(mode: "semantic")
get_users() = http.get("users")
"#;
        let tokens = tokenize(source).unwrap();
        let program = parse(tokens).unwrap();

        if let Definition::FuncDef(f) = &program.definitions[0] {
            let config = f.self_heal.as_ref().unwrap();
            assert_eq!(config.mode, HealMode::Semantic);
        } else {
            panic!("Expected function definition");
        }
    }

    #[test]
    fn test_function_without_self_heal() {
        let source = r#"+http
normal_func() = 42
"#;
        let tokens = tokenize(source).unwrap();
        let program = parse(tokens).unwrap();

        if let Definition::FuncDef(f) = &program.definitions[0] {
            assert!(f.self_heal.is_none());
        } else {
            panic!("Expected function definition");
        }
    }

    #[test]
    fn test_looks_like_function_def_with_self_heal() {
        let tokens = tokenize("@self_heal\nmain = 42").unwrap();
        assert!(looks_like_function_def(&tokens));

        let tokens = tokenize("@self_heal(max_attempts: 3)\nmain = 42").unwrap();
        assert!(looks_like_function_def(&tokens));
    }

    #[test]
    fn test_parse_simple_invariant() {
        let source = r#"+http
invariant api_url != "https://production.com"
main = 42
"#;
        let tokens = tokenize(source).unwrap();
        let program = parse(tokens).unwrap();

        assert_eq!(program.definitions.len(), 2); // 1 invariant + 1 function
        if let Definition::Invariant(expr) = &program.definitions[0] {
            // Should be a BinaryOp with NotEq
            assert!(matches!(expr, Expr::BinaryOp { op: BinaryOp::NotEq, .. }));
        } else {
            panic!("Expected invariant definition, got {:?}", program.definitions[0]);
        }
    }

    #[test]
    fn test_parse_multiple_invariants() {
        let source = r#"+http
invariant api_url != "https://prod.example.com"
invariant response != "mock_response"
goal "safe API calls"
main = 42
"#;
        let tokens = tokenize(source).unwrap();
        let program = parse(tokens).unwrap();

        assert_eq!(program.definitions.len(), 4); // 2 invariants + 1 goal + 1 function

        // First should be invariant
        assert!(matches!(&program.definitions[0], Definition::Invariant(_)));
        // Second should be invariant
        assert!(matches!(&program.definitions[1], Definition::Invariant(_)));
        // Third should be goal
        assert!(matches!(&program.definitions[2], Definition::Goal(_)));
        // Fourth should be function
        assert!(matches!(&program.definitions[3], Definition::FuncDef(_)));
    }

    #[test]
    fn test_parse_invariant_with_negation() {
        let source = r#"+http
invariant !contains(source, "mock_data")
main = 42
"#;
        let tokens = tokenize(source).unwrap();
        let program = parse(tokens).unwrap();

        if let Definition::Invariant(expr) = &program.definitions[0] {
            // Should be a UnaryOp with Not
            assert!(matches!(expr, Expr::UnaryOp { op: UnaryOp::Not, .. }));
        } else {
            panic!("Expected invariant definition");
        }
    }

    #[test]
    fn test_parse_invariant_with_comparison() {
        let source = r#"+http
invariant max_retries <= 5
main = 42
"#;
        let tokens = tokenize(source).unwrap();
        let program = parse(tokens).unwrap();

        if let Definition::Invariant(expr) = &program.definitions[0] {
            // Should be a BinaryOp with LtEq
            assert!(matches!(expr, Expr::BinaryOp { op: BinaryOp::LtEq, .. }));
        } else {
            panic!("Expected invariant definition");
        }
    }
}
