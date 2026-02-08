use logos::Logos;
use serde::{Deserialize, Serialize};

/// Tokens de AURA
/// Diseñados para ser mínimos y no ambiguos
#[derive(Logos, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[logos(skip r"[ \t]+")]  // Ignorar espacios y tabs (no newlines)
pub enum Token {
    // ═══════════════════════════════════════════════════════════
    // CAPACIDADES (+http, +json, etc.)
    // ═══════════════════════════════════════════════════════════
    #[token("+")]
    Plus,

    // ═══════════════════════════════════════════════════════════
    // DEFINICIONES (@User, @Post, etc.)
    // ═══════════════════════════════════════════════════════════
    #[token("@")]
    At,

    // ═══════════════════════════════════════════════════════════
    // TIPOS PRIMITIVOS
    // ═══════════════════════════════════════════════════════════
    #[token(":i")]
    TypeInt,

    #[token(":f")]
    TypeFloat,

    #[token(":s")]
    TypeString,

    #[token(":b")]
    TypeBool,

    #[token(":ts")]
    TypeTimestamp,

    #[token(":uuid")]
    TypeUuid,

    // ═══════════════════════════════════════════════════════════
    // DELIMITADORES
    // ═══════════════════════════════════════════════════════════
    #[token("{")]
    LBrace,

    #[token("}")]
    RBrace,

    #[token("[")]
    LBracket,

    #[token("]")]
    RBracket,

    #[token("(")]
    LParen,

    #[token(")")]
    RParen,

    #[token(":")]
    Colon,

    #[token(",")]
    Comma,

    #[token(".")]
    Dot,

    #[token("\n")]
    Newline,

    #[token(";")]
    Semicolon,

    // ═══════════════════════════════════════════════════════════
    // OPERADORES
    // ═══════════════════════════════════════════════════════════
    #[token("=")]
    Eq,

    #[token("==")]
    EqEq,

    #[token("!=")]
    NotEq,

    #[token("<")]
    Lt,

    #[token(">")]
    Gt,

    #[token("<=")]
    LtEq,

    #[token(">=")]
    GtEq,

    #[token("->")]
    Arrow,

    #[token("|>")]
    PipeOp,

    #[token("|")]
    Pipe,

    #[token("?")]
    Question,

    #[token("!")]
    Bang,

    #[token("&")]
    Ampersand,

    #[token("++")]
    PlusPlus,

    #[token("-")]
    Minus,

    #[token("*")]
    Star,

    #[token("/", priority = 1)]
    Slash,

    #[token("%")]
    Percent,

    #[token("??")]
    NullCoalesce,

    #[token("?.")]
    SafeNav,

    // ═══════════════════════════════════════════════════════════
    // PALABRAS CLAVE
    // ═══════════════════════════════════════════════════════════
    #[token("goal")]
    Goal,

    #[token("true")]
    True,

    #[token("false")]
    False,

    #[token("nil")]
    Nil,

    #[token("if")]
    If,

    #[token("else")]
    Else,

    #[token("for")]
    For,

    #[token("in")]
    In,

    #[token("while")]
    While,

    #[token("return")]
    Return,

    #[token("break")]
    Break,

    #[token("continue")]
    Continue,

    #[token("expect")]
    Expect,

    #[token("invariant")]
    Invariant,

    // ═══════════════════════════════════════════════════════════
    // ESPECIALES AURA
    // ═══════════════════════════════════════════════════════════
    #[token("#test")]
    TestMarker,

    #[token("#doc")]
    DocMarker,

    #[token("+crud")]
    CrudMarker,

    #[token("+api")]
    ApiMarker,

    #[token("+job")]
    JobMarker,

    #[token("+ws")]
    WsMarker,

    #[token("+agent")]
    AgentMarker,

    #[token("+append")]
    AppendMarker,

    #[token("+runtime")]
    RuntimeMarker,

    // ═══════════════════════════════════════════════════════════
    // ANOTACIONES
    // ═══════════════════════════════════════════════════════════
    #[token("@pk")]
    AnnPk,

    #[token("@unique")]
    AnnUnique,

    #[token("@email")]
    AnnEmail,

    #[token("@url")]
    AnnUrl,

    #[token("@hash")]
    AnnHash,

    #[token("@hide")]
    AnnHide,

    #[token("@auto")]
    AnnAuto,

    #[token("@rel")]
    AnnRel,

    #[token("@index")]
    AnnIndex,

    #[token("@auth")]
    AnnAuth,

    #[token("@own")]
    AnnOwn,

    #[token("@me")]
    AnnMe,

    #[token("@body")]
    AnnBody,

    #[token("@id")]
    AnnId,

    #[token("@env")]
    AnnEnv,

    #[token("@on")]
    AnnOn,

    #[token("@every")]
    AnnEvery,

    #[token("@self_heal")]
    AnnSelfHeal,

    // ═══════════════════════════════════════════════════════════
    // ANOTACIONES CON PARÁMETROS (se parsean después)
    // ═══════════════════════════════════════════════════════════
    #[token("@min")]
    AnnMin,

    #[token("@max")]
    AnnMax,

    #[token("@range")]
    AnnRange,

    #[token("@match")]
    AnnMatch,

    #[token("@role")]
    AnnRole,

    // ═══════════════════════════════════════════════════════════
    // HTTP METHODS (para +api)
    // ═══════════════════════════════════════════════════════════
    #[token("GET")]
    HttpGet,

    #[token("POST")]
    HttpPost,

    #[token("PUT")]
    HttpPut,

    #[token("PATCH")]
    HttpPatch,

    #[token("DEL")]
    HttpDelete,

    // ═══════════════════════════════════════════════════════════
    // LITERALS
    // ═══════════════════════════════════════════════════════════
    #[regex(r#""([^"\\]|\\.)*""#, |lex| {
        let s = lex.slice();
        let inner = &s[1..s.len()-1];
        // Process escape sequences
        let mut result = String::new();
        let mut chars = inner.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '\\' {
                match chars.next() {
                    Some('n') => result.push('\n'),
                    Some('t') => result.push('\t'),
                    Some('r') => result.push('\r'),
                    Some('\\') => result.push('\\'),
                    Some('"') => result.push('"'),
                    Some(other) => {
                        result.push('\\');
                        result.push(other);
                    }
                    None => result.push('\\'),
                }
            } else {
                result.push(c);
            }
        }
        result
    })]
    String(String),

    #[regex(r"[0-9]+\.[0-9]+", |lex| lex.slice().parse::<f64>().ok())]
    Float(f64),

    #[regex(r"[0-9]+", |lex| lex.slice().parse::<i64>().ok(), priority = 2)]
    Int(i64),

    // ═══════════════════════════════════════════════════════════
    // IDENTIFICADORES
    // ═══════════════════════════════════════════════════════════
    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*", |lex| lex.slice().to_string(), priority = 1)]
    Ident(String),

    // ═══════════════════════════════════════════════════════════
    // PLACEHOLDER
    // ═══════════════════════════════════════════════════════════
    #[token("_", priority = 2)]
    Underscore,

    // ═══════════════════════════════════════════════════════════
    // SPREAD
    // ═══════════════════════════════════════════════════════════
    #[token("..")]
    Spread,

    // ═══════════════════════════════════════════════════════════
    // COMENTARIOS
    // ═══════════════════════════════════════════════════════════
    #[regex(r"#[^\n]*", |lex| lex.slice().to_string())]
    Comment(String),

    // ═══════════════════════════════════════════════════════════
    // PATH (para rutas API como /users/:id)
    // ═══════════════════════════════════════════════════════════
    #[regex(r"/[a-zA-Z0-9_/:*-]+", |lex| lex.slice().to_string(), priority = 2)]
    Path(String),
}

impl Token {
    /// Retorna true si el token es un operador
    pub fn is_operator(&self) -> bool {
        matches!(
            self,
            Token::Plus
                | Token::Minus
                | Token::Star
                | Token::Slash
                | Token::Percent
                | Token::EqEq
                | Token::NotEq
                | Token::Lt
                | Token::Gt
                | Token::LtEq
                | Token::GtEq
                | Token::Ampersand
                | Token::Pipe
                | Token::PlusPlus
        )
    }

    /// Retorna true si el token es una palabra clave
    pub fn is_keyword(&self) -> bool {
        matches!(
            self,
            Token::Goal
                | Token::True
                | Token::False
                | Token::Nil
                | Token::If
                | Token::Else
                | Token::For
                | Token::In
                | Token::While
                | Token::Return
                | Token::Break
                | Token::Continue
                | Token::Expect
                | Token::Invariant
        )
    }

    /// Retorna true si el token es un tipo primitivo
    pub fn is_primitive_type(&self) -> bool {
        matches!(
            self,
            Token::TypeInt
                | Token::TypeFloat
                | Token::TypeString
                | Token::TypeBool
                | Token::TypeTimestamp
                | Token::TypeUuid
        )
    }

    /// Retorna true si el token es una anotación
    pub fn is_annotation(&self) -> bool {
        matches!(
            self,
            Token::AnnPk
                | Token::AnnUnique
                | Token::AnnEmail
                | Token::AnnUrl
                | Token::AnnHash
                | Token::AnnHide
                | Token::AnnAuto
                | Token::AnnRel
                | Token::AnnIndex
                | Token::AnnAuth
                | Token::AnnOwn
                | Token::AnnMin
                | Token::AnnMax
                | Token::AnnRange
                | Token::AnnMatch
                | Token::AnnRole
                | Token::AnnSelfHeal
        )
    }

    /// Retorna true si el token es una anotación de función
    pub fn is_function_annotation(&self) -> bool {
        matches!(self, Token::AnnSelfHeal)
    }
}

/// Información de posición para errores
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }
}

/// Token con información de posición
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Spanned<T> {
    pub value: T,
    pub span: Span,
}

impl<T> Spanned<T> {
    pub fn new(value: T, span: Span) -> Self {
        Self { value, span }
    }
}
