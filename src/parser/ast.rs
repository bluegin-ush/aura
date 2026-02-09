use serde::{Deserialize, Serialize};
use crate::lexer::Span;

/// Programa AURA completo
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Program {
    pub capabilities: Vec<Capability>,
    pub definitions: Vec<Definition>,
}

/// Capacidad habilitada (+http, +json, etc.)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Capability {
    pub name: String,
    pub span: Span,
}

/// Definición de nivel superior
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Definition {
    TypeDef(TypeDef),
    EnumDef(EnumDef),
    FuncDef(FuncDef),
    ApiDef(ApiDef),
    TestDef(TestDef),
    /// Goal declaration - metadata that describes intent, optionally with a check expression
    Goal(GoalDef),
    /// Invariant - constraint that healing cannot violate
    /// Invariants are checked before applying any fix
    Invariant(Expr),
    /// Observe declaration at top level
    Observe(ObserveDef),
}

/// Goal definition with optional active check expression
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GoalDef {
    /// Human-readable description of the goal
    pub description: String,
    /// Optional check expression that evaluates to bool
    /// When present, the goal is "active" and evaluated continuously
    pub check: Option<Expr>,
    pub span: Span,
}

/// Observe definition at top level
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ObserveDef {
    /// Target expression to observe
    pub target: String,
    /// Optional condition for filtering observations
    pub condition: Option<Expr>,
    pub span: Span,
}

/// Definición de tipo (@User { ... })
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TypeDef {
    pub name: String,
    pub fields: Vec<Field>,
    pub annotations: Vec<Annotation>,
    pub span: Span,
}

/// Campo de un tipo
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Field {
    pub name: String,
    pub ty: Type,
    pub nullable: bool,
    pub default: Option<Expr>,
    pub annotations: Vec<Annotation>,
    pub span: Span,
}

/// Tipo
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Type {
    Int,
    Float,
    String,
    Bool,
    Timestamp,
    Uuid,
    Named(String),
    List(Box<Type>),
    Map(Box<Type>, Box<Type>),
    Optional(Box<Type>),
}

/// Anotación (@pk, @min(5), etc.)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Annotation {
    pub name: String,
    pub args: Vec<Expr>,
    pub span: Span,
}

/// Definición de enum (@Status = pending | active | done)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnumDef {
    pub name: String,
    pub variants: Vec<EnumVariant>,
    pub span: Span,
}

/// Variante de enum
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnumVariant {
    pub name: String,
    pub fields: Option<Vec<Type>>,
}

/// Configuración de self-healing para una función
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SelfHealConfig {
    /// Número máximo de intentos de reparación (default: 3)
    pub max_attempts: u32,
    /// Modo de healing
    pub mode: HealMode,
}

impl Default for SelfHealConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            mode: HealMode::Auto,
        }
    }
}

/// Modo de healing
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum HealMode {
    /// Healing técnico - corrige errores de sintaxis y tipos
    Technical,
    /// Healing semántico - corrige basándose en los goals
    Semantic,
    /// Automático - elige el mejor modo según el error
    Auto,
}

impl HealMode {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "technical" => HealMode::Technical,
            "semantic" => HealMode::Semantic,
            _ => HealMode::Auto,
        }
    }
}

/// Definición de función
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FuncDef {
    pub name: String,
    pub has_effect: bool,  // Marcado con !
    pub params: Vec<Param>,
    pub return_type: Option<Type>,
    pub body: Expr,
    pub span: Span,
    /// Configuración de self-healing (si tiene @self_heal)
    pub self_heal: Option<SelfHealConfig>,
}

/// Parámetro de función
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Param {
    pub name: String,
    pub ty: Option<Type>,
}

/// Definición de API (+api("/v1"): ...)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApiDef {
    pub base_path: String,
    pub routes: Vec<Route>,
    pub span: Span,
}

/// Ruta de API
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Route {
    pub method: HttpMethod,
    pub path: String,
    pub handler: Expr,
    pub annotations: Vec<Annotation>,
}

/// Método HTTP
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
}

/// Definición de test (#test nombre: expr)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TestDef {
    pub name: String,
    pub expr: Expr,
    pub span: Span,
}

/// Expresión
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Expr {
    // Literales
    Int(i64),
    Float(f64),
    String(String),
    Bool(bool),
    Nil,

    // Identificador
    Ident(String),

    // Placeholder (_)
    Placeholder,

    // Lista [a, b, c]
    List(Vec<Expr>),

    // Record {a: 1, b: 2}
    Record(Vec<(String, Expr)>),

    // Acceso a campo (expr.field)
    FieldAccess(Box<Expr>, String),

    // Safe navigation (expr?.field)
    SafeAccess(Box<Expr>, String),

    // Llamada a función (func(args))
    Call {
        func: Box<Expr>,
        args: Vec<Expr>,
        has_effect: bool,
    },

    // Operación binaria (a + b)
    BinaryOp {
        left: Box<Expr>,
        op: BinaryOp,
        right: Box<Expr>,
    },

    // Operación unaria (!a, -a)
    UnaryOp {
        op: UnaryOp,
        expr: Box<Expr>,
    },

    // Pipe (a | b | c)
    Pipe(Vec<Expr>),

    // Pattern matching (expr | Pat -> expr | Pat -> expr)
    Match {
        expr: Box<Expr>,
        arms: Vec<MatchArm>,
    },

    // Lambda (x -> expr)
    Lambda {
        params: Vec<String>,
        body: Box<Expr>,
    },

    // Bloque (: expr1 expr2 ... exprN)
    Block(Vec<Expr>),

    // Let binding (name = expr)
    Let {
        name: String,
        value: Box<Expr>,
    },

    // If expression
    If {
        condition: Box<Expr>,
        then_branch: Box<Expr>,
        else_branch: Option<Box<Expr>>,
    },

    // For loop
    For {
        var: String,
        iter: Box<Expr>,
        body: Box<Expr>,
    },

    // Interpolated string
    InterpolatedString(Vec<StringPart>),

    // Spread (...expr)
    Spread(Box<Expr>),

    // Null coalesce (a ?? b)
    NullCoalesce(Box<Expr>, Box<Expr>),

    // Expect - intent verification (expect condition "optional message")
    // If condition is false, registers as expectation failure (not a crash)
    Expect {
        condition: Box<Expr>,
        message: Option<String>,
    },

    // Observe - declare a variable/expression to be monitored
    Observe {
        target: String,
        condition: Option<Box<Expr>>,
    },

    // Reason - explicit deliberation block
    Reason {
        observations: Vec<Expr>,
        question: String,
    },
}

/// Parte de un string interpolado
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum StringPart {
    Literal(String),
    Expr(Box<Expr>),
}

/// Operador binario
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    NotEq,
    Lt,
    Gt,
    LtEq,
    GtEq,
    And,
    Or,
    Concat,
}

/// Operador unario
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum UnaryOp {
    Not,
    Neg,
}

/// Brazo de pattern matching
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub body: Expr,
}

/// Patrón para matching
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Pattern {
    Wildcard,
    Ident(String),
    Literal(Expr),
    Constructor { name: String, fields: Vec<Pattern> },
}
