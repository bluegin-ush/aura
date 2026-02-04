//! Ollama Provider - Integración con instancias locales de Ollama
//!
//! Este módulo implementa `AgentProvider` usando la API de Ollama
//! para self-healing con modelos LLM locales.
//!
//! ## Ejemplo de uso
//!
//! ```ignore
//! use aura::agent::{OllamaProvider, AgentRequest, EventType};
//!
//! let provider = OllamaProvider::new()
//!     .with_model("llama3.2")
//!     .with_base_url("http://localhost:11434");
//!
//! let request = AgentRequest::error("x + 1", "main.aura", 1, 1)
//!     .with_message("Variable 'x' no definida");
//!
//! let response = provider.send_request(request).await?;
//! ```
//!
//! ## Requisitos
//!
//! - Ollama debe estar corriendo localmente (o en la URL configurada)
//! - Un modelo compatible debe estar disponible (ej: llama3.2, codellama)

use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use super::bridge::{AgentError, AgentProvider};
use super::request::AgentRequest;
use super::response::{AgentResponse, Patch, Suggestion};

/// Modelo por defecto de Ollama
const DEFAULT_MODEL: &str = "llama3.2";

/// URL base por defecto de Ollama
const DEFAULT_BASE_URL: &str = "http://localhost:11434";

/// Timeout por defecto para requests (60 segundos - modelos locales pueden ser lentos)
const DEFAULT_TIMEOUT_SECS: u64 = 60;

/// System prompt para healing de AURA (mismo estilo que Claude)
const AURA_HEALING_SYSTEM_PROMPT: &str = r#"Eres un agente de reparación para AURA, un lenguaje de programación diseñado para agentes IA.

Cuando recibes un error de AURA, tu trabajo es:
1. Analizar el contexto del código
2. Identificar la causa raíz del error
3. Proponer un fix con alta confianza

Sintaxis de AURA:
- Capacidades: +http +json +db (en lugar de imports)
- Tipos inline: @User { name:s age:i email:s? }
- Funciones: add(a b) = a + b
- Efectos (IO): fetch!(url) indica side effects
- Pipes: data | filter(_.active) | map(_.name)
- Pattern matching: result | Ok(v) -> v | Err(e) -> nil
- Null coalescing: user?.name ?? "Anonymous"

Responde SIEMPRE en JSON válido con este formato exacto:
{
    "action": "patch" | "generate" | "suggest" | "clarify" | "escalate",
    "patch": {
        "old_code": "código original a reemplazar",
        "new_code": "código corregido"
    },
    "explanation": "Explicación clara de por qué este fix funciona",
    "confidence": 0.0-1.0
}

Reglas importantes:
- Si puedes arreglar el error con certeza, usa "action": "patch"
- Si necesitas generar código nuevo, usa "action": "generate"
- Si no estás seguro, usa "action": "suggest" con varias opciones
- Si necesitas más información, usa "action": "clarify" con preguntas
- Si el problema es muy complejo, usa "action": "escalate"
- El campo "confidence" debe reflejar tu certeza real (0.8+ para auto-apply)
- IMPORTANTE: Responde SOLO con el JSON, sin texto adicional antes o después
"#;

/// Proveedor de Ollama para self-healing con modelos locales
pub struct OllamaProvider {
    /// URL base de la API de Ollama
    base_url: String,
    /// Modelo a usar
    model: String,
    /// Cliente HTTP
    client: reqwest::Client,
    /// Timeout para requests
    timeout: Duration,
    /// Habilitar streaming (no usado actualmente, reservado para futuro)
    #[allow(dead_code)]
    streaming: bool,
}

impl OllamaProvider {
    /// Crea un nuevo proveedor de Ollama con valores por defecto
    pub fn new() -> Self {
        Self {
            base_url: DEFAULT_BASE_URL.to_string(),
            model: DEFAULT_MODEL.to_string(),
            client: reqwest::Client::new(),
            timeout: Duration::from_secs(DEFAULT_TIMEOUT_SECS),
            streaming: false,
        }
    }

    /// Configura el modelo a usar
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    /// Configura la URL base de Ollama
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    /// Configura el timeout para requests
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Habilita o deshabilita streaming (reservado para futuro uso)
    pub fn with_streaming(mut self, streaming: bool) -> Self {
        self.streaming = streaming;
        self
    }

    /// Formatea un AgentRequest como prompt para Ollama
    fn format_prompt(&self, request: &AgentRequest) -> String {
        let mut prompt = String::new();

        // Agregar información del error
        prompt.push_str("## Error en AURA\n\n");

        if let Some(msg) = &request.message {
            prompt.push_str(&format!("**Mensaje de error:** {}\n\n", msg));
        }

        // Agregar código
        prompt.push_str(&format!(
            "**Código:**\n```aura\n{}\n```\n\n",
            request.context.source
        ));

        // Agregar ubicación
        prompt.push_str(&format!(
            "**Ubicación:** {}:{}:{}\n\n",
            request.location.file, request.location.line, request.location.column
        ));

        // Agregar código circundante si existe
        if let Some(ref surrounding) = request.context.surrounding_code {
            if !surrounding.is_empty() {
                prompt.push_str(&format!(
                    "**Contexto adicional:**\n```aura\n{}\n```\n\n",
                    surrounding
                ));
            }
        }

        // Agregar intentos previos si existen
        if !request.previous_attempts.is_empty() {
            prompt.push_str("**Intentos previos que no funcionaron:**\n");
            for attempt in &request.previous_attempts {
                prompt.push_str(&format!("- `{}`\n", attempt));
            }
            prompt.push('\n');
        }

        prompt.push_str(
            "Por favor, analiza el error y proporciona un fix en el formato JSON especificado.",
        );

        prompt
    }

    /// Parsea la respuesta JSON de Ollama a AgentResponse
    fn parse_response(&self, json_str: &str) -> Result<AgentResponse, AgentError> {
        // Intentar extraer JSON del texto (el modelo a veces agrega texto antes/después)
        let json_str = self.extract_json(json_str)?;

        let parsed: OllamaHealingResponse = serde_json::from_str(&json_str).map_err(|e| {
            AgentError::SerializationError(format!("Error parseando respuesta: {}", e))
        })?;

        // Convertir a AgentResponse
        let response = match parsed.action.as_str() {
            "patch" => {
                let patch = parsed.patch.ok_or_else(|| {
                    AgentError::SerializationError(
                        "Respuesta 'patch' sin patch incluido".to_string(),
                    )
                })?;

                AgentResponse::patch(
                    Patch::new(&patch.old_code, &patch.new_code),
                    &parsed.explanation,
                    parsed.confidence,
                )
            }
            "generate" => {
                let code = parsed.generated_code.ok_or_else(|| {
                    AgentError::SerializationError("Respuesta 'generate' sin código".to_string())
                })?;

                AgentResponse::generate(code, &parsed.explanation, parsed.confidence)
            }
            "suggest" => {
                let suggestions = parsed
                    .suggestions
                    .unwrap_or_default()
                    .into_iter()
                    .map(|s| Suggestion {
                        code: s.code,
                        rationale: s.rationale,
                        confidence: s.confidence,
                    })
                    .collect();

                AgentResponse::suggest(suggestions, &parsed.explanation)
            }
            "clarify" => {
                let questions = parsed.questions.unwrap_or_default();
                AgentResponse::clarify(questions, &parsed.explanation)
            }
            "escalate" => AgentResponse::escalate(
                parsed
                    .escalation_reason
                    .unwrap_or_else(|| "Error complejo".to_string()),
                &parsed.explanation,
            ),
            other => {
                return Err(AgentError::SerializationError(format!(
                    "Acción desconocida: {}",
                    other
                )));
            }
        };

        Ok(response)
    }

    /// Extrae JSON de un texto que puede tener contenido adicional
    fn extract_json(&self, text: &str) -> Result<String, AgentError> {
        // Buscar el inicio de un objeto JSON
        if let Some(start) = text.find('{') {
            // Encontrar el cierre correspondiente
            let mut depth = 0;
            let mut end = start;

            for (i, c) in text[start..].char_indices() {
                match c {
                    '{' => depth += 1,
                    '}' => {
                        depth -= 1;
                        if depth == 0 {
                            end = start + i + 1;
                            break;
                        }
                    }
                    _ => {}
                }
            }

            if depth == 0 {
                return Ok(text[start..end].to_string());
            }
        }

        Err(AgentError::SerializationError(
            "No se encontró JSON válido en la respuesta".to_string(),
        ))
    }

    /// Llama a la API de Ollama usando el endpoint /api/generate
    async fn call_api(&self, prompt: &str) -> Result<String, AgentError> {
        let url = format!("{}/api/generate", self.base_url);

        let request_body = OllamaGenerateRequest {
            model: self.model.clone(),
            prompt: prompt.to_string(),
            system: AURA_HEALING_SYSTEM_PROMPT.to_string(),
            stream: false,
            options: Some(OllamaOptions {
                temperature: Some(0.2), // Baja temperatura para respuestas más determinísticas
                num_predict: Some(4096),
            }),
        };

        let response = self
            .client
            .post(&url)
            .header("content-type", "application/json")
            .timeout(self.timeout)
            .json(&request_body)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    AgentError::Timeout {
                        timeout_ms: self.timeout.as_millis() as u64,
                    }
                } else if e.is_connect() {
                    AgentError::ConnectionError(format!(
                        "No se pudo conectar a Ollama en {}: {}. ¿Está Ollama corriendo?",
                        self.base_url, e
                    ))
                } else {
                    AgentError::ConnectionError(format!("Error de red: {}", e))
                }
            })?;

        let status = response.status();

        if !status.is_success() {
            let error_text: String = response.text().await.unwrap_or_default();
            return Err(AgentError::InvalidResponse(format!(
                "Error de Ollama ({}): {}",
                status.as_u16(),
                error_text
            )));
        }

        let api_response: OllamaGenerateResponse = response.json().await.map_err(|e| {
            AgentError::SerializationError(format!("Error parseando respuesta de Ollama: {}", e))
        })?;

        Ok(api_response.response)
    }

    /// Verifica si Ollama está corriendo y el modelo está disponible
    async fn check_availability(&self) -> Result<bool, AgentError> {
        let url = format!("{}/api/tags", self.base_url);

        let response = self
            .client
            .get(&url)
            .timeout(Duration::from_secs(5))
            .send()
            .await
            .map_err(|e| {
                AgentError::ConnectionError(format!("No se pudo conectar a Ollama: {}", e))
            })?;

        if !response.status().is_success() {
            return Ok(false);
        }

        let tags: OllamaTagsResponse = response.json().await.map_err(|e| {
            AgentError::SerializationError(format!("Error parseando respuesta de tags: {}", e))
        })?;

        // Verificar si el modelo configurado está disponible
        Ok(tags
            .models
            .iter()
            .any(|m| m.name == self.model || m.name.starts_with(&format!("{}:", self.model))))
    }
}

impl Default for OllamaProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentProvider for OllamaProvider {
    fn send_request<'a>(
        &'a self,
        request: AgentRequest,
    ) -> Pin<Box<dyn Future<Output = Result<AgentResponse, AgentError>> + Send + 'a>> {
        Box::pin(async move {
            let prompt = self.format_prompt(&request);
            let response_text = self.call_api(&prompt).await?;
            self.parse_response(&response_text)
        })
    }

    fn name(&self) -> &str {
        "ollama"
    }

    fn is_available<'a>(&'a self) -> Pin<Box<dyn Future<Output = bool> + Send + 'a>> {
        Box::pin(async move { self.check_availability().await.unwrap_or(false) })
    }
}

// ============================================================================
// Estructuras para serialización de la API de Ollama
// ============================================================================

/// Request para el endpoint /api/generate
#[derive(Serialize)]
struct OllamaGenerateRequest {
    model: String,
    prompt: String,
    system: String,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    options: Option<OllamaOptions>,
}

/// Opciones de generación para Ollama
#[derive(Serialize)]
struct OllamaOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    num_predict: Option<usize>,
}

/// Response del endpoint /api/generate
#[derive(Deserialize)]
struct OllamaGenerateResponse {
    response: String,
    #[allow(dead_code)]
    model: String,
    #[allow(dead_code)]
    done: bool,
}

/// Response del endpoint /api/tags (para listar modelos)
#[derive(Deserialize)]
struct OllamaTagsResponse {
    models: Vec<OllamaModel>,
}

/// Información de un modelo en Ollama
#[derive(Deserialize)]
struct OllamaModel {
    name: String,
    #[allow(dead_code)]
    #[serde(default)]
    size: u64,
}

// ============================================================================
// Estructuras para la respuesta de healing
// ============================================================================

#[derive(Deserialize)]
struct OllamaHealingResponse {
    action: String,
    #[serde(default)]
    patch: Option<OllamaPatch>,
    #[serde(default)]
    generated_code: Option<String>,
    #[serde(default)]
    suggestions: Option<Vec<OllamaSuggestion>>,
    #[serde(default)]
    questions: Option<Vec<String>>,
    #[serde(default)]
    escalation_reason: Option<String>,
    #[serde(default)]
    explanation: String,
    #[serde(default)]
    confidence: f32,
}

#[derive(Deserialize)]
struct OllamaPatch {
    old_code: String,
    new_code: String,
}

#[derive(Deserialize)]
struct OllamaSuggestion {
    code: String,
    rationale: String,
    confidence: f32,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::Action;

    fn create_test_provider() -> OllamaProvider {
        OllamaProvider::new()
    }

    #[test]
    fn test_default_configuration() {
        let provider = OllamaProvider::new();
        assert_eq!(provider.base_url, "http://localhost:11434");
        assert_eq!(provider.model, "llama3.2");
        assert_eq!(provider.timeout, Duration::from_secs(60));
    }

    #[test]
    fn test_builder_pattern() {
        let provider = OllamaProvider::new()
            .with_model("codellama")
            .with_base_url("http://192.168.1.100:11434")
            .with_timeout(Duration::from_secs(120))
            .with_streaming(true);

        assert_eq!(provider.model, "codellama");
        assert_eq!(provider.base_url, "http://192.168.1.100:11434");
        assert_eq!(provider.timeout, Duration::from_secs(120));
        assert!(provider.streaming);
    }

    #[test]
    fn test_extract_json_clean() {
        let provider = create_test_provider();
        let text = r#"{"action": "patch", "confidence": 0.9}"#;
        let result = provider.extract_json(text);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), text);
    }

    #[test]
    fn test_extract_json_with_surrounding_text() {
        let provider = create_test_provider();
        let text = r#"Here is the JSON response:
{"action": "patch", "explanation": "fix", "confidence": 0.9}
I hope this helps!"#;
        let result = provider.extract_json(text);
        assert!(result.is_ok());
        let json = result.unwrap();
        assert!(json.starts_with('{'));
        assert!(json.ends_with('}'));
        assert!(json.contains("patch"));
    }

    #[test]
    fn test_extract_json_nested() {
        let provider = create_test_provider();
        let text =
            r#"{"action": "patch", "patch": {"old_code": "x", "new_code": "y"}, "confidence": 0.9}"#;
        let result = provider.extract_json(text);
        assert!(result.is_ok());
        let json = result.unwrap();
        assert!(json.contains("old_code"));
        assert!(json.contains("new_code"));
    }

    #[test]
    fn test_extract_json_no_json() {
        let provider = create_test_provider();
        let text = "No JSON here, just plain text!";
        let result = provider.extract_json(text);
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_json_unclosed() {
        let provider = create_test_provider();
        let text = r#"{"action": "patch", "confidence": 0.9"#; // Missing closing brace
        let result = provider.extract_json(text);
        assert!(result.is_err());
    }

    #[test]
    fn test_format_prompt_basic() {
        let provider = create_test_provider();
        let request = AgentRequest::error("x + 1", "test.aura", 5, 1)
            .with_message("Variable 'x' no definida");

        let prompt = provider.format_prompt(&request);

        assert!(prompt.contains("Variable 'x' no definida"));
        assert!(prompt.contains("x + 1"));
        assert!(prompt.contains("test.aura"));
        assert!(prompt.contains("5:1")); // line:column
    }

    #[test]
    fn test_format_prompt_with_previous_attempts() {
        let provider = create_test_provider();
        let request = AgentRequest::error("x + 1", "test.aura", 1, 1)
            .with_message("Variable no definida")
            .with_previous_attempt("let x = nil")
            .with_previous_attempt("let x = undefined");

        let prompt = provider.format_prompt(&request);

        assert!(prompt.contains("Intentos previos"));
        assert!(prompt.contains("let x = nil"));
        assert!(prompt.contains("let x = undefined"));
    }

    #[test]
    fn test_parse_patch_response() {
        let provider = create_test_provider();
        let json = r#"{
            "action": "patch",
            "patch": {
                "old_code": "x + 1",
                "new_code": "let x = 0; x + 1"
            },
            "explanation": "Declarar variable x",
            "confidence": 0.95
        }"#;

        let result = provider.parse_response(json);
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response.action, Action::Patch);
        assert!(response.patch.is_some());
        assert_eq!(response.confidence, 0.95);

        let patch = response.patch.unwrap();
        assert_eq!(patch.old_code, "x + 1");
        assert_eq!(patch.new_code, "let x = 0; x + 1");
    }

    #[test]
    fn test_parse_generate_response() {
        let provider = create_test_provider();
        let json = r#"{
            "action": "generate",
            "generated_code": "fn fibonacci(n) = n <= 1 ? n : fibonacci(n-1) + fibonacci(n-2)",
            "explanation": "Implementación de Fibonacci",
            "confidence": 0.85
        }"#;

        let result = provider.parse_response(json);
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response.action, Action::Generate);
        assert!(response.generated_code.is_some());
        assert!(response.generated_code.unwrap().contains("fibonacci"));
    }

    #[test]
    fn test_parse_suggest_response() {
        let provider = create_test_provider();
        let json = r#"{
            "action": "suggest",
            "suggestions": [
                {"code": "let x = 0", "rationale": "Inicializar a cero", "confidence": 0.8},
                {"code": "let x = nil", "rationale": "Usar nil como valor inicial", "confidence": 0.6}
            ],
            "explanation": "Varias opciones posibles",
            "confidence": 0.7
        }"#;

        let result = provider.parse_response(json);
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response.action, Action::Suggest);
        assert_eq!(response.suggestions.len(), 2);
    }

    #[test]
    fn test_parse_clarify_response() {
        let provider = create_test_provider();
        let json = r#"{
            "action": "clarify",
            "questions": [
                "¿Qué tipo debería tener la variable x?",
                "¿Es x un número o una cadena?"
            ],
            "explanation": "Necesito más información",
            "confidence": 0.0
        }"#;

        let result = provider.parse_response(json);
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response.action, Action::Clarify);
        assert_eq!(response.questions.len(), 2);
    }

    #[test]
    fn test_parse_escalate_response() {
        let provider = create_test_provider();
        let json = r#"{
            "action": "escalate",
            "escalation_reason": "Problema de arquitectura complejo",
            "explanation": "Requiere análisis manual",
            "confidence": 0.0
        }"#;

        let result = provider.parse_response(json);
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response.action, Action::Escalate);
        assert!(response.escalation_reason.is_some());
    }

    #[test]
    fn test_parse_response_unknown_action() {
        let provider = create_test_provider();
        let json = r#"{
            "action": "unknown_action",
            "explanation": "Test",
            "confidence": 0.5
        }"#;

        let result = provider.parse_response(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_response_missing_patch() {
        let provider = create_test_provider();
        let json = r#"{
            "action": "patch",
            "explanation": "Fix sin patch",
            "confidence": 0.9
        }"#;

        let result = provider.parse_response(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_response_with_surrounding_text() {
        let provider = create_test_provider();
        let text = r#"Let me analyze this error...

{
    "action": "patch",
    "patch": {
        "old_code": "x",
        "new_code": "let x = 0"
    },
    "explanation": "Define x",
    "confidence": 0.9
}

This should fix the issue."#;

        let result = provider.parse_response(text);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().action, Action::Patch);
    }

    #[test]
    fn test_provider_name() {
        let provider = OllamaProvider::new();
        assert_eq!(provider.name(), "ollama");
    }

    #[test]
    fn test_default_trait() {
        let provider = OllamaProvider::default();
        assert_eq!(provider.model, DEFAULT_MODEL);
        assert_eq!(provider.base_url, DEFAULT_BASE_URL);
    }
}
