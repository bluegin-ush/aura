//! Modulo de errores de AURA.
//!
//! Proporciona tipos de error estructurados para:
//! - Errores de sintaxis (E0xx)
//! - Errores de tipos (E1xx)
//! - Errores de referencias (E2xx)
//! - Errores de efectos (E3xx)
//! - Errores de runtime (E4xx)
//! - Errores de capacidades (E5xx)
//! - Errores de agente (E9xx)
//!
//! ## Formatos de salida
//!
//! - JSON estructurado para agentes IA (metodo `to_json()`)
//! - Formato bonito con colores para humanos (modulo `pretty`)

pub mod pretty;

use serde::{Deserialize, Serialize};
use crate::lexer::Span;

pub use pretty::{
    format_error_pretty,
    format_errors_pretty,
    format_parse_error,
    format_type_error,
    format_reference_error,
    format_capability_error,
    format_effect_error,
    ErrorType,
};

/// Codigo de error AURA
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

/// Ubicacion del error
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Location {
    pub file: String,
    pub line: usize,
    pub col: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_col: Option<usize>,
    /// Span original en bytes (opcional, para formateo preciso)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub span: Option<Span>,
}

impl Location {
    /// Crea una ubicacion simple sin span
    pub fn simple(file: impl Into<String>, line: usize, col: usize) -> Self {
        Self {
            file: file.into(),
            line,
            col,
            end_col: None,
            span: None,
        }
    }

    /// Crea una ubicacion con rango de columnas
    pub fn with_range(file: impl Into<String>, line: usize, col: usize, end_col: usize) -> Self {
        Self {
            file: file.into(),
            line,
            col,
            end_col: Some(end_col),
            span: None,
        }
    }

    /// Crea una ubicacion a partir de un span y el codigo fuente
    pub fn from_span(span: &Span, source: &str, file: &str) -> Self {
        let before = &source[..span.start.min(source.len())];
        let line = before.lines().count().max(1);
        let last_newline = before.rfind('\n').map(|i| i + 1).unwrap_or(0);
        let col = span.start.saturating_sub(last_newline) + 1;
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
            span: Some(span.clone()),
        }
    }

    /// Obtiene el span si esta disponible, o lo calcula desde linea/columna
    pub fn get_span(&self, source: &str) -> Span {
        if let Some(ref span) = self.span {
            span.clone()
        } else {
            // Calcular span desde linea y columna
            let mut current_line = 1;
            let mut line_start = 0;

            for (i, c) in source.char_indices() {
                if current_line == self.line {
                    line_start = i;
                    break;
                }
                if c == '\n' {
                    current_line += 1;
                }
            }

            let start = (line_start + self.col.saturating_sub(1)).min(source.len());
            let end = if let Some(end_col) = self.end_col {
                (line_start + end_col.saturating_sub(1)).min(source.len())
            } else {
                (start + 1).min(source.len())
            };

            Span::new(start, end)
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
    /// Crea un nuevo error con los datos basicos
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

    /// Crea un error de sintaxis
    pub fn syntax(span: &Span, source: &str, file: &str, message: impl Into<String>) -> Self {
        Self::new(
            ErrorCode::syntax(1),
            Severity::Error,
            Location::from_span(span, source, file),
            message,
        )
    }

    /// Crea un error de tipo
    pub fn type_error(span: &Span, source: &str, file: &str, message: impl Into<String>) -> Self {
        Self::new(
            ErrorCode::type_error(1),
            Severity::Error,
            Location::from_span(span, source, file),
            message,
        )
    }

    /// Crea un error de referencia (variable/funcion no definida)
    pub fn reference_error(span: &Span, source: &str, file: &str, name: &str) -> Self {
        Self::new(
            ErrorCode::reference(1),
            Severity::Error,
            Location::from_span(span, source, file),
            format!("'{}' no esta definido", name),
        )
    }

    /// Crea un error de efecto no manejado
    pub fn effect_error(span: &Span, source: &str, file: &str, message: impl Into<String>) -> Self {
        Self::new(
            ErrorCode::effect(1),
            Severity::Error,
            Location::from_span(span, source, file),
            message,
        )
    }

    /// Crea un error de runtime
    pub fn runtime_error(span: &Span, source: &str, file: &str, message: impl Into<String>) -> Self {
        Self::new(
            ErrorCode::runtime(1),
            Severity::Error,
            Location::from_span(span, source, file),
            message,
        )
    }

    /// Crea un error de capacidad faltante
    pub fn capability_error(span: &Span, source: &str, file: &str, capability: &str) -> Self {
        Self::new(
            ErrorCode::capability(1),
            Severity::Error,
            Location::from_span(span, source, file),
            format!("Capacidad '+{}' requerida pero no declarada", capability),
        )
        .with_suggestion(
            format!("Agrega '+{}' al inicio del archivo", capability),
            Some(format!("+{}", capability)),
        )
    }

    /// Crea un error de agente
    pub fn agent_error(span: &Span, source: &str, file: &str, message: impl Into<String>) -> Self {
        Self::new(
            ErrorCode::agent(1),
            Severity::Error,
            Location::from_span(span, source, file),
            message,
        )
    }

    /// Agrega detalles adicionales al error
    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(details);
        self
    }

    /// Agrega una sugerencia de correccion
    pub fn with_suggestion(mut self, message: impl Into<String>, replacement: Option<String>) -> Self {
        self.suggestion = Some(Suggestion {
            message: message.into(),
            replacement,
        });
        self
    }

    /// Agrega lineas de contexto
    pub fn with_context(mut self, context: Vec<ContextLine>) -> Self {
        self.context = context;
        self
    }

    /// Agrega una nota a los detalles
    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        let note_value = serde_json::json!({ "note": note.into() });
        self.details = Some(match self.details {
            Some(serde_json::Value::Object(mut map)) => {
                if let serde_json::Value::Object(note_map) = note_value {
                    map.extend(note_map);
                }
                serde_json::Value::Object(map)
            }
            _ => note_value,
        });
        self
    }

    /// Serializa el error a JSON (para agentes IA)
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    /// Formatea el error de forma bonita con colores (para humanos)
    ///
    /// # Argumentos
    ///
    /// * `source` - El codigo fuente completo
    ///
    /// # Retorna
    ///
    /// Una cadena con el error formateado con colores y contexto
    pub fn to_pretty(&self, source: &str) -> String {
        format_error_pretty(self, source, &self.location.file)
    }
}

/// Colección de errores
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Errors {
    pub errors: Vec<AuraError>,
}

impl Errors {
    /// Crea una coleccion vacia de errores
    pub fn new() -> Self {
        Self { errors: Vec::new() }
    }

    /// Agrega un error a la coleccion
    pub fn push(&mut self, error: AuraError) {
        self.errors.push(error);
    }

    /// Verifica si la coleccion esta vacia
    pub fn is_empty(&self) -> bool {
        self.errors.is_empty()
    }

    /// Verifica si hay errores (no solo warnings)
    pub fn has_errors(&self) -> bool {
        self.errors.iter().any(|e| e.severity == Severity::Error)
    }

    /// Cuenta el numero de errores
    pub fn error_count(&self) -> usize {
        self.errors.iter().filter(|e| e.severity == Severity::Error).count()
    }

    /// Cuenta el numero de advertencias
    pub fn warning_count(&self) -> usize {
        self.errors.iter().filter(|e| e.severity == Severity::Warning).count()
    }

    /// Serializa todos los errores a JSON (para agentes IA)
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(&self.errors).unwrap_or_default()
    }

    /// Formatea todos los errores de forma bonita (para humanos)
    ///
    /// # Argumentos
    ///
    /// * `source` - El codigo fuente completo
    /// * `filename` - Nombre del archivo
    ///
    /// # Retorna
    ///
    /// Todos los errores formateados con colores y un resumen final
    pub fn to_pretty(&self, source: &str, filename: &str) -> String {
        format_errors_pretty(&self.errors, source, filename)
    }

    /// Itera sobre los errores
    pub fn iter(&self) -> impl Iterator<Item = &AuraError> {
        self.errors.iter()
    }

    /// Obtiene el numero total de errores y advertencias
    pub fn len(&self) -> usize {
        self.errors.len()
    }
}
