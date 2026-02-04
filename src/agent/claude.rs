//! Claude API Provider - Integración con la API de Anthropic
//!
//! Este módulo implementa `AgentProvider` usando la API de Claude
//! para self-healing real con IA.
//!
//! ## Ejemplo de uso
//!
//! ```ignore
//! use aura::agent::{ClaudeProvider, AgentRequest, EventType};
//!
//! let provider = ClaudeProvider::new("sk-ant-api-key")
//!     .with_model("claude-3-5-sonnet-20241022");
//!
//! let request = AgentRequest::error("x + 1", "main.aura", 1, 1)
//!     .with_message("Variable 'x' no definida");
//!
//! let response = provider.send_request(request).await?;
//! ```

use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use super::bridge::{AgentProvider, AgentError};
use super::request::AgentRequest;
use super::response::{AgentResponse, Patch, Suggestion};

/// Modelo por defecto de Claude
const DEFAULT_MODEL: &str = "claude-sonnet-4-20250514";

/// URL base de la API de Anthropic
const API_BASE_URL: &str = "https://api.anthropic.com/v1";

/// Timeout por defecto para requests (30 segundos)
const DEFAULT_TIMEOUT_SECS: u64 = 30;

/// Máximo de tokens en la respuesta
const DEFAULT_MAX_TOKENS: usize = 4096;

use super::prompts;

/// Proveedor de Claude para self-healing
#[cfg(feature = "claude-api")]
pub struct ClaudeProvider {
    /// API key de Anthropic
    api_key: String,
    /// Modelo a usar
    model: String,
    /// URL base de la API
    base_url: String,
    /// Cliente HTTP
    client: reqwest::Client,
    /// Timeout para requests
    timeout: Duration,
    /// Máximo de tokens en respuesta
    max_tokens: usize,
}

#[cfg(feature = "claude-api")]
impl ClaudeProvider {
    /// Crea un nuevo proveedor de Claude
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            model: DEFAULT_MODEL.to_string(),
            base_url: API_BASE_URL.to_string(),
            client: reqwest::Client::new(),
            timeout: Duration::from_secs(DEFAULT_TIMEOUT_SECS),
            max_tokens: DEFAULT_MAX_TOKENS,
        }
    }

    /// Configura el modelo a usar
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    /// Configura la URL base (para testing)
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    /// Configura el timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Configura el máximo de tokens
    pub fn with_max_tokens(mut self, max_tokens: usize) -> Self {
        self.max_tokens = max_tokens;
        self
    }

    /// Formatea un AgentRequest como prompt para Claude
    fn format_prompt(&self, request: &AgentRequest) -> String {
        let mut prompt = String::new();

        // Agregar información del error
        prompt.push_str(&format!("## Error en AURA\n\n"));

        if let Some(msg) = &request.message {
            prompt.push_str(&format!("**Mensaje de error:** {}\n\n", msg));
        }

        // Agregar código
        prompt.push_str(&format!("**Código:**\n```aura\n{}\n```\n\n", request.context.source));

        // Agregar ubicación
        prompt.push_str(&format!(
            "**Ubicación:** {}:{}:{}\n\n",
            request.location.file,
            request.location.line,
            request.location.column
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

        prompt.push_str("Por favor, analiza el error y proporciona un fix en el formato JSON especificado.");

        prompt
    }

    /// Parsea la respuesta JSON de Claude a AgentResponse
    fn parse_response(&self, json_str: &str) -> Result<AgentResponse, AgentError> {
        // Intentar extraer JSON del texto (Claude a veces agrega texto antes/después)
        let json_str = self.extract_json(json_str)?;

        let parsed: ClaudeHealingResponse = serde_json::from_str(&json_str)
            .map_err(|e| AgentError::SerializationError(format!("Error parseando respuesta: {}", e)))?;

        // Convertir a AgentResponse
        let response = match parsed.action.as_str() {
            "patch" => {
                let patch = parsed.patch.ok_or_else(|| AgentError::SerializationError("Respuesta 'patch' sin patch incluido".to_string()))?;

                AgentResponse::patch(
                    Patch::new(&patch.old_code, &patch.new_code),
                    &parsed.explanation,
                    parsed.confidence,
                )
            }
            "generate" => {
                let code = parsed.generated_code.ok_or_else(|| AgentError::SerializationError("Respuesta 'generate' sin código".to_string()))?;

                AgentResponse::generate(code, &parsed.explanation, parsed.confidence)
            }
            "suggest" => {
                let suggestions = parsed.suggestions.unwrap_or_default()
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
            "escalate" => {
                AgentResponse::escalate(
                    parsed.escalation_reason.unwrap_or_else(|| "Error complejo".to_string()),
                    &parsed.explanation,
                )
            }
            other => {
                return Err(AgentError::SerializationError(format!("Acción desconocida: {}", other)));
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

        Err(AgentError::SerializationError("No se encontró JSON válido en la respuesta".to_string()))
    }

    /// Llama a la API de Claude
    async fn call_api(&self, prompt: &str) -> Result<String, AgentError> {
        let url = format!("{}/messages", self.base_url);

        let request_body = ClaudeApiRequest {
            model: self.model.clone(),
            max_tokens: self.max_tokens,
            system: prompts::healing_system_prompt(),
            messages: vec![ClaudeMessage {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
        };

        let response = self.client
            .post(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .timeout(self.timeout)
            .json(&request_body)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    AgentError::Timeout { timeout_ms: self.timeout.as_millis() as u64 }
                } else if e.is_connect() {
                    AgentError::ConnectionError(e.to_string())
                } else {
                    AgentError::ConnectionError(format!("Network error: {}", e))
                }
            })?;

        let status = response.status();

        if status == 429 {
            return Err(AgentError::RateLimited {
                retry_after_ms: Some(60000), // Default 1 minute
            });
        }

        if !status.is_success() {
            let error_text: String = response.text().await.unwrap_or_default();
            return Err(AgentError::InvalidResponse(
                format!("API error ({}): {}", status.as_u16(), error_text)
            ));
        }

        let api_response: ClaudeApiResponse = response.json().await.map_err(|e| {
            AgentError::SerializationError(format!("Error parseando respuesta de API: {}", e))
        })?;

        // Extraer el texto de la respuesta
        api_response.content
            .into_iter()
            .find(|c| c.content_type == "text")
            .map(|c| c.text)
            .ok_or_else(|| AgentError::SerializationError("Respuesta sin contenido de texto".to_string()))
    }
}

#[cfg(feature = "claude-api")]
impl AgentProvider for ClaudeProvider {
    fn send_request(
        &self,
        request: AgentRequest,
    ) -> Pin<Box<dyn Future<Output = Result<AgentResponse, AgentError>> + Send + '_>> {
        Box::pin(async move {
            let prompt = self.format_prompt(&request);
            let response_text = self.call_api(&prompt).await?;
            self.parse_response(&response_text)
        })
    }

    fn name(&self) -> &str {
        "claude"
    }

    fn is_available(&self) -> Pin<Box<dyn Future<Output = bool> + Send + '_>> {
        Box::pin(async move {
            // Verificar que tenemos API key
            !self.api_key.is_empty()
        })
    }
}

// Estructuras para serialización de la API de Claude

#[derive(Serialize)]
struct ClaudeApiRequest {
    model: String,
    max_tokens: usize,
    system: String,
    messages: Vec<ClaudeMessage>,
}

#[derive(Serialize, Deserialize)]
struct ClaudeMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct ClaudeApiResponse {
    content: Vec<ClaudeContent>,
    #[allow(dead_code)]
    model: String,
    #[allow(dead_code)]
    stop_reason: Option<String>,
}

#[derive(Deserialize)]
struct ClaudeContent {
    #[serde(rename = "type")]
    content_type: String,
    text: String,
}

// Estructuras para la respuesta de healing

#[derive(Deserialize)]
struct ClaudeHealingResponse {
    action: String,
    patch: Option<ClaudePatch>,
    generated_code: Option<String>,
    suggestions: Option<Vec<ClaudeSuggestion>>,
    questions: Option<Vec<String>>,
    escalation_reason: Option<String>,
    explanation: String,
    confidence: f32,
}

#[derive(Deserialize)]
struct ClaudePatch {
    old_code: String,
    new_code: String,
}

#[derive(Deserialize)]
struct ClaudeSuggestion {
    code: String,
    rationale: String,
    confidence: f32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::Action;

    #[test]
    fn test_extract_json_clean() {
        let provider = create_test_provider();
        let text = r#"{"action": "patch", "confidence": 0.9}"#;
        let result = provider.extract_json(text);
        assert!(result.is_ok());
    }

    #[test]
    fn test_extract_json_with_text() {
        let provider = create_test_provider();
        let text = r#"Here is my response:
{"action": "patch", "explanation": "fix", "confidence": 0.9}
Hope this helps!"#;
        let result = provider.extract_json(text);
        assert!(result.is_ok());
        assert!(result.unwrap().starts_with('{'));
    }

    #[test]
    fn test_extract_json_nested() {
        let provider = create_test_provider();
        let text = r#"{"action": "patch", "patch": {"old": "x", "new": "y"}, "confidence": 0.9}"#;
        let result = provider.extract_json(text);
        assert!(result.is_ok());
        assert!(result.unwrap().contains("patch"));
    }

    #[test]
    fn test_extract_json_no_json() {
        let provider = create_test_provider();
        let text = "No JSON here!";
        let result = provider.extract_json(text);
        assert!(result.is_err());
    }

    #[test]
    fn test_format_prompt() {
        let provider = create_test_provider();
        let request = AgentRequest::error("x + 1", "test.aura", 5, 1)
            .with_message("Variable 'x' no definida");

        let prompt = provider.format_prompt(&request);

        assert!(prompt.contains("Variable 'x' no definida"));
        assert!(prompt.contains("x + 1"));
        assert!(prompt.contains("test.aura"));
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
    }

    #[test]
    fn test_parse_escalate_response() {
        let provider = create_test_provider();
        let json = r#"{
            "action": "escalate",
            "escalation_reason": "Problema muy complejo",
            "explanation": "Requiere análisis manual",
            "confidence": 0.0
        }"#;

        let result = provider.parse_response(json);
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response.action, Action::Escalate);
    }

    #[cfg(feature = "claude-api")]
    fn create_test_provider() -> ClaudeProvider {
        ClaudeProvider::new("test-key")
    }

    #[cfg(not(feature = "claude-api"))]
    fn create_test_provider() -> MockClaudeProvider {
        MockClaudeProvider
    }

    // Mock para tests sin feature
    #[cfg(not(feature = "claude-api"))]
    struct MockClaudeProvider;

    #[cfg(not(feature = "claude-api"))]
    impl MockClaudeProvider {
        fn extract_json(&self, text: &str) -> Result<String, AgentError> {
            if let Some(start) = text.find('{') {
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
            Err(AgentError::SerializationError("No JSON found".to_string()))
        }

        fn format_prompt(&self, request: &AgentRequest) -> String {
            format!(
                "Error: {:?}\nCode: {}\nFile: {}",
                request.message,
                request.context.code,
                request.location.file
            )
        }

        fn parse_response(&self, json_str: &str) -> Result<AgentResponse, AgentError> {
            let json_str = self.extract_json(json_str)?;
            let parsed: ClaudeHealingResponse = serde_json::from_str(&json_str)
                .map_err(|e| AgentError::SerializationError(format!("Parse error: {}", e)))?;

            match parsed.action.as_str() {
                "patch" => {
                    let patch = parsed.patch.ok_or_else(|| AgentError::SerializationError("No patch".to_string()))?;
                    Ok(AgentResponse::patch(
                        Patch::new(&patch.old_code, &patch.new_code),
                        &parsed.explanation,
                        parsed.confidence,
                    ))
                }
                "escalate" => {
                    Ok(AgentResponse::escalate(
                        parsed.escalation_reason.unwrap_or_default(),
                        &parsed.explanation,
                    ))
                }
                _ => Err(AgentError::SerializationError("Unknown action".to_string())),
            }
        }
    }
}
