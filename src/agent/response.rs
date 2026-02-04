//! Estructuras para respuestas del agente IA

use serde::{Deserialize, Serialize};

/// Tipo de acción que el agente recomienda
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    /// Aplicar un parche al código existente
    Patch,
    /// Generar código nuevo
    Generate,
    /// Sugerir cambios sin aplicarlos automáticamente
    Suggest,
    /// Pedir clarificación al usuario
    Clarify,
    /// Escalar a un humano (problema muy complejo o ambiguo)
    Escalate,
}

/// Un parche de código (old -> new)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Patch {
    /// Código original a reemplazar
    pub old_code: String,
    /// Código nuevo que lo reemplaza
    pub new_code: String,
    /// Ubicación donde aplicar el parche
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<PatchLocation>,
    /// Descripción del cambio
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl Patch {
    pub fn new(old_code: impl Into<String>, new_code: impl Into<String>) -> Self {
        Self {
            old_code: old_code.into(),
            new_code: new_code.into(),
            location: None,
            description: None,
        }
    }

    pub fn with_location(mut self, file: impl Into<String>, start_line: usize, end_line: usize) -> Self {
        self.location = Some(PatchLocation {
            file: file.into(),
            start_line,
            end_line,
        });
        self
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }
}

/// Ubicación donde aplicar un parche
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PatchLocation {
    pub file: String,
    pub start_line: usize,
    pub end_line: usize,
}

/// Sugerencia de código sin parche automático
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Suggestion {
    /// Código sugerido
    pub code: String,
    /// Por qué se sugiere este cambio
    pub rationale: String,
    /// Nivel de confianza en la sugerencia (0.0 - 1.0)
    pub confidence: f32,
}

/// Respuesta del agente IA
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentResponse {
    /// Acción recomendada
    pub action: Action,

    /// Parche a aplicar (si action == Patch)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub patch: Option<Patch>,

    /// Código generado (si action == Generate)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generated_code: Option<String>,

    /// Sugerencias (si action == Suggest)
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub suggestions: Vec<Suggestion>,

    /// Explicación de la respuesta (siempre presente)
    pub explanation: String,

    /// Nivel de confianza en la respuesta (0.0 - 1.0)
    pub confidence: f32,

    /// Preguntas de clarificación (si action == Clarify)
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub questions: Vec<String>,

    /// Razón de escalación (si action == Escalate)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub escalation_reason: Option<String>,

    /// Metadatos adicionales
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<ResponseMetadata>,
}

/// Metadatos de la respuesta
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseMetadata {
    /// ID del modelo que generó la respuesta
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_id: Option<String>,
    /// Tokens usados en la solicitud
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tokens_used: Option<usize>,
    /// Tiempo de procesamiento en ms
    #[serde(skip_serializing_if = "Option::is_none")]
    pub processing_time_ms: Option<u64>,
    /// ID de sesión
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
}

impl AgentResponse {
    /// Crea una respuesta de parche
    pub fn patch(patch: Patch, explanation: impl Into<String>, confidence: f32) -> Self {
        Self {
            action: Action::Patch,
            patch: Some(patch),
            generated_code: None,
            suggestions: Vec::new(),
            explanation: explanation.into(),
            confidence: confidence.clamp(0.0, 1.0),
            questions: Vec::new(),
            escalation_reason: None,
            metadata: None,
        }
    }

    /// Crea una respuesta de generación de código
    pub fn generate(code: impl Into<String>, explanation: impl Into<String>, confidence: f32) -> Self {
        Self {
            action: Action::Generate,
            patch: None,
            generated_code: Some(code.into()),
            suggestions: Vec::new(),
            explanation: explanation.into(),
            confidence: confidence.clamp(0.0, 1.0),
            questions: Vec::new(),
            escalation_reason: None,
            metadata: None,
        }
    }

    /// Crea una respuesta con sugerencias
    pub fn suggest(suggestions: Vec<Suggestion>, explanation: impl Into<String>) -> Self {
        let avg_confidence = if suggestions.is_empty() {
            0.5
        } else {
            suggestions.iter().map(|s| s.confidence).sum::<f32>() / suggestions.len() as f32
        };

        Self {
            action: Action::Suggest,
            patch: None,
            generated_code: None,
            suggestions,
            explanation: explanation.into(),
            confidence: avg_confidence,
            questions: Vec::new(),
            escalation_reason: None,
            metadata: None,
        }
    }

    /// Crea una respuesta pidiendo clarificación
    pub fn clarify(questions: Vec<String>, explanation: impl Into<String>) -> Self {
        Self {
            action: Action::Clarify,
            patch: None,
            generated_code: None,
            suggestions: Vec::new(),
            explanation: explanation.into(),
            confidence: 0.0,
            questions,
            escalation_reason: None,
            metadata: None,
        }
    }

    /// Crea una respuesta de escalación
    pub fn escalate(reason: impl Into<String>, explanation: impl Into<String>) -> Self {
        Self {
            action: Action::Escalate,
            patch: None,
            generated_code: None,
            suggestions: Vec::new(),
            explanation: explanation.into(),
            confidence: 0.0,
            questions: Vec::new(),
            escalation_reason: Some(reason.into()),
            metadata: None,
        }
    }

    pub fn with_metadata(mut self, metadata: ResponseMetadata) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Deserializa una respuesta desde JSON
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Serializa la respuesta a JSON
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Verifica si la respuesta tiene alta confianza (>= 0.8)
    pub fn is_high_confidence(&self) -> bool {
        self.confidence >= 0.8
    }

    /// Verifica si la respuesta es aplicable automáticamente
    pub fn is_auto_applicable(&self) -> bool {
        matches!(self.action, Action::Patch | Action::Generate) && self.is_high_confidence()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_patch_response() {
        let patch = Patch::new("x + 1", "let x = 0; x + 1")
            .with_description("Declarar variable x antes de usarla");

        let response = AgentResponse::patch(
            patch,
            "La variable 'x' no estaba definida. Se agregó su declaración.",
            0.95,
        );

        assert_eq!(response.action, Action::Patch);
        assert!(response.is_high_confidence());
        assert!(response.is_auto_applicable());
    }

    #[test]
    fn test_response_deserialization() {
        let json = r#"{
            "action": "patch",
            "patch": {
                "old_code": "x + 1",
                "new_code": "let x = 0; x + 1"
            },
            "explanation": "Variable no definida",
            "confidence": 0.9
        }"#;

        let response = AgentResponse::from_json(json).unwrap();
        assert_eq!(response.action, Action::Patch);
        assert!(response.patch.is_some());
        assert_eq!(response.confidence, 0.9);
    }

    #[test]
    fn test_clarify_response() {
        let response = AgentResponse::clarify(
            vec![
                "Qué tipo debería tener la variable 'x'?".to_string(),
                "Es 'x' un número o una cadena?".to_string(),
            ],
            "Necesito más información para resolver el error",
        );

        assert_eq!(response.action, Action::Clarify);
        assert_eq!(response.questions.len(), 2);
        assert!(!response.is_auto_applicable());
    }

    #[test]
    fn test_generate_response() {
        let response = AgentResponse::generate(
            "fn fibonacci(n: int) -> int {\n  ? n <= 1 -> n\n  _ -> fibonacci(n-1) + fibonacci(n-2)\n}",
            "Implementación recursiva de Fibonacci",
            0.85,
        );

        assert_eq!(response.action, Action::Generate);
        assert!(response.generated_code.is_some());
        assert!(response.is_high_confidence());
    }

    #[test]
    fn test_escalate_response() {
        let response = AgentResponse::escalate(
            "El código tiene dependencias circulares complejas que requieren restructuración manual",
            "Este problema es demasiado complejo para resolución automática",
        );

        assert_eq!(response.action, Action::Escalate);
        assert!(response.escalation_reason.is_some());
        assert!(!response.is_auto_applicable());
    }
}
