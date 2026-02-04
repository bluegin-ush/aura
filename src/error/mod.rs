use serde::{Deserialize, Serialize};
use crate::lexer::Span;

/// Código de error AURA
///
/// Categorías:
/// - E0xx: Sintaxis
/// - E1xx: Tipos
/// - E2xx: Referencias
/// - E3xx: Efectos
/// - E4xx: Runtime
/// - E5xx: Capacidades
/// - E9xx: Agente
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ErrorCode(pub String);

impl ErrorCode {
    pub fn syntax(n: u8) -> Self {
        Self(format!("E0{:02}", n))
    }

    pub fn type_error(n: u8) -> Self {
        Self(format!("E1{:02}", n))
    }

    pub fn reference(n: u8) -> Self {
        Self(format!("E2{:02}", n))
    }

    pub fn effect(n: u8) -> Self {
        Self(format!("E3{:02}", n))
    }

    pub fn runtime(n: u8) -> Self {
        Self(format!("E4{:02}", n))
    }

    pub fn capability(n: u8) -> Self {
        Self(format!("E5{:02}", n))
    }

    pub fn agent(n: u8) -> Self {
        Self(format!("E9{:02}", n))
    }
}

/// Severidad del error
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
    Info,
}

/// Sugerencia de corrección para agentes
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Suggestion {
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replacement: Option<String>,
}

/// Línea de contexto
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContextLine {
    pub line: usize,
    pub code: String,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub highlight: bool,
}

/// Ubicación del error
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Location {
    pub file: String,
    pub line: usize,
    pub col: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_col: Option<usize>,
}

impl Location {
    /// Crea una ubicación a partir de un span y el código fuente
    pub fn from_span(span: &Span, source: &str, file: &str) -> Self {
        let before = &source[..span.start];
        let line = before.lines().count().max(1);
        let last_newline = before.rfind('\n').map(|i| i + 1).unwrap_or(0);
        let col = span.start - last_newline + 1;
        let end_col = if span.end > span.start {
            Some(col + (span.end - span.start))
        } else {
            None
        };

        Self {
            file: file.to_string(),
            line,
            col,
            end_col,
        }
    }
}

/// Error estructurado de AURA (JSON-friendly para agentes)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuraError {
    pub code: ErrorCode,
    pub severity: Severity,
    pub location: Location,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<Suggestion>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub context: Vec<ContextLine>,
}

impl AuraError {
    pub fn new(code: ErrorCode, severity: Severity, location: Location, message: impl Into<String>) -> Self {
        Self {
            code,
            severity,
            location,
            message: message.into(),
            details: None,
            suggestion: None,
            context: Vec::new(),
        }
    }

    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(details);
        self
    }

    pub fn with_suggestion(mut self, message: impl Into<String>, replacement: Option<String>) -> Self {
        self.suggestion = Some(Suggestion {
            message: message.into(),
            replacement,
        });
        self
    }

    pub fn with_context(mut self, context: Vec<ContextLine>) -> Self {
        self.context = context;
        self
    }

    /// Serializa el error a JSON
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }
}

/// Colección de errores
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Errors {
    pub errors: Vec<AuraError>,
}

impl Errors {
    pub fn new() -> Self {
        Self { errors: Vec::new() }
    }

    pub fn push(&mut self, error: AuraError) {
        self.errors.push(error);
    }

    pub fn is_empty(&self) -> bool {
        self.errors.is_empty()
    }

    pub fn has_errors(&self) -> bool {
        self.errors.iter().any(|e| e.severity == Severity::Error)
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(&self.errors).unwrap_or_default()
    }
}
