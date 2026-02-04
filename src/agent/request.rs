//! Estructuras para solicitudes al agente IA

use serde::{Deserialize, Serialize};

/// Tipo de evento que dispara la solicitud al agente
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    /// Error de compilación o runtime que el agente debe resolver
    Error,
    /// Código faltante que el agente debe generar (stubs, implementaciones)
    Missing,
    /// Problema de rendimiento que el agente debe optimizar
    Performance,
    /// Solicitud de expansión de funcionalidad
    Expansion,
}

/// Contexto del código relevante para la solicitud
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Context {
    /// Código fuente relevante (fragmento o archivo completo)
    pub source: String,

    /// Tipos involucrados (si aplica)
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub types: Vec<TypeInfo>,

    /// Estado del runtime (variables, valores)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runtime_state: Option<serde_json::Value>,

    /// Imports/dependencias disponibles
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub available_imports: Vec<String>,

    /// Código circundante para más contexto
    #[serde(skip_serializing_if = "Option::is_none")]
    pub surrounding_code: Option<String>,
}

impl Context {
    pub fn new(source: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            types: Vec::new(),
            runtime_state: None,
            available_imports: Vec::new(),
            surrounding_code: None,
        }
    }

    pub fn with_types(mut self, types: Vec<TypeInfo>) -> Self {
        self.types = types;
        self
    }

    pub fn with_runtime_state(mut self, state: serde_json::Value) -> Self {
        self.runtime_state = Some(state);
        self
    }

    pub fn with_imports(mut self, imports: Vec<String>) -> Self {
        self.available_imports = imports;
        self
    }

    pub fn with_surrounding(mut self, code: impl Into<String>) -> Self {
        self.surrounding_code = Some(code.into());
        self
    }
}

/// Información de un tipo
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TypeInfo {
    pub name: String,
    pub definition: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub constraints: Option<String>,
}

impl TypeInfo {
    pub fn new(name: impl Into<String>, definition: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            definition: definition.into(),
            constraints: None,
        }
    }
}

/// Ubicación en el código fuente
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SourceLocation {
    /// Nombre del archivo
    pub file: String,
    /// Línea (1-indexed)
    pub line: usize,
    /// Columna (1-indexed)
    pub column: usize,
    /// Línea final (para rangos)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_line: Option<usize>,
    /// Columna final (para rangos)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_column: Option<usize>,
}

impl SourceLocation {
    pub fn new(file: impl Into<String>, line: usize, column: usize) -> Self {
        Self {
            file: file.into(),
            line,
            column,
            end_line: None,
            end_column: None,
        }
    }

    pub fn with_end(mut self, end_line: usize, end_column: usize) -> Self {
        self.end_line = Some(end_line);
        self.end_column = Some(end_column);
        self
    }
}

/// Restricciones para la respuesta del agente
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Constraints {
    /// Máximo de tokens en la respuesta
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<usize>,

    /// Timeout en milisegundos
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,

    /// Temperatura para generación (0.0 = determinístico, 1.0 = creativo)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,

    /// Solo respuestas que no modifiquen la semántica
    #[serde(default)]
    pub preserve_semantics: bool,

    /// Restricciones de estilo de código
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style_guide: Option<String>,
}

impl Default for Constraints {
    fn default() -> Self {
        Self {
            max_tokens: Some(4096),
            timeout_ms: Some(30000),
            temperature: Some(0.2),
            preserve_semantics: true,
            style_guide: None,
        }
    }
}

impl Constraints {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_max_tokens(mut self, tokens: usize) -> Self {
        self.max_tokens = Some(tokens);
        self
    }

    pub fn with_timeout(mut self, ms: u64) -> Self {
        self.timeout_ms = Some(ms);
        self
    }

    pub fn with_temperature(mut self, temp: f32) -> Self {
        self.temperature = Some(temp.clamp(0.0, 1.0));
        self
    }

    pub fn creative() -> Self {
        Self {
            temperature: Some(0.8),
            preserve_semantics: false,
            ..Default::default()
        }
    }

    pub fn strict() -> Self {
        Self {
            temperature: Some(0.0),
            preserve_semantics: true,
            ..Default::default()
        }
    }
}

/// Solicitud al agente IA
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentRequest {
    /// Tipo de evento que dispara la solicitud
    pub event_type: EventType,

    /// Contexto del código
    pub context: Context,

    /// Ubicación en el código fuente
    pub location: SourceLocation,

    /// Restricciones para la respuesta
    #[serde(default)]
    pub constraints: Constraints,

    /// Mensaje adicional o instrucciones específicas
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,

    /// Errores previos (para evitar repetir soluciones fallidas)
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub previous_attempts: Vec<String>,

    /// ID de sesión para mantener contexto entre solicitudes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
}

impl AgentRequest {
    /// Crea una nueva solicitud con valores mínimos
    pub fn new(event_type: EventType) -> Self {
        Self {
            event_type,
            context: Context::new(""),
            location: SourceLocation::new("unknown", 1, 1),
            constraints: Constraints::default(),
            message: None,
            previous_attempts: Vec::new(),
            session_id: None,
        }
    }

    /// Crea una solicitud para un error
    pub fn error(source: impl Into<String>, file: impl Into<String>, line: usize, col: usize) -> Self {
        Self {
            event_type: EventType::Error,
            context: Context::new(source),
            location: SourceLocation::new(file, line, col),
            constraints: Constraints::default(),
            message: None,
            previous_attempts: Vec::new(),
            session_id: None,
        }
    }

    /// Crea una solicitud para código faltante
    pub fn missing(source: impl Into<String>, file: impl Into<String>, line: usize, col: usize) -> Self {
        Self {
            event_type: EventType::Missing,
            context: Context::new(source),
            location: SourceLocation::new(file, line, col),
            constraints: Constraints::default(),
            message: None,
            previous_attempts: Vec::new(),
            session_id: None,
        }
    }

    pub fn with_context(mut self, source: impl Into<String>) -> Self {
        self.context = Context::new(source);
        self
    }

    pub fn with_full_context(mut self, context: Context) -> Self {
        self.context = context;
        self
    }

    pub fn with_location(mut self, file: impl Into<String>, line: usize, col: usize) -> Self {
        self.location = SourceLocation::new(file, line, col);
        self
    }

    pub fn with_constraints(mut self, constraints: Constraints) -> Self {
        self.constraints = constraints;
        self
    }

    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }

    pub fn with_previous_attempt(mut self, attempt: impl Into<String>) -> Self {
        self.previous_attempts.push(attempt.into());
        self
    }

    pub fn with_session(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    /// Serializa la solicitud a JSON
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Serializa la solicitud a JSON compacto
    pub fn to_json_compact(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_serialization() {
        let request = AgentRequest::error("fn main() { x + 1 }", "main.aura", 1, 15)
            .with_message("Variable 'x' no definida");

        let json = request.to_json().unwrap();
        assert!(json.contains("error"));
        assert!(json.contains("main.aura"));
        assert!(json.contains("Variable 'x' no definida"));
    }

    #[test]
    fn test_request_with_types() {
        let context = Context::new("let user: User = get_user()")
            .with_types(vec![
                TypeInfo::new("User", "{ name: str, age: int }"),
            ]);

        let request = AgentRequest::new(EventType::Missing)
            .with_full_context(context)
            .with_location("api.aura", 10, 5);

        let json = request.to_json().unwrap();
        assert!(json.contains("User"));
        assert!(json.contains("name: str"));
    }

    #[test]
    fn test_constraints() {
        let strict = Constraints::strict();
        assert_eq!(strict.temperature, Some(0.0));
        assert!(strict.preserve_semantics);

        let creative = Constraints::creative();
        assert_eq!(creative.temperature, Some(0.8));
        assert!(!creative.preserve_semantics);
    }
}
