//! OpenAI API Provider - Integration with OpenAI, Azure OpenAI, and compatible APIs
//!
//! This module implements `AgentProvider` using the OpenAI API
//! for self-healing with GPT-4 and compatible models.
//!
//! ## Example usage
//!
//! ```ignore
//! use aura::agent::{OpenAIProvider, AgentRequest, EventType};
//!
//! // Standard OpenAI
//! let provider = OpenAIProvider::new("sk-...")
//!     .with_model("gpt-4");
//!
//! // Azure OpenAI
//! let azure_provider = OpenAIProvider::new("azure-api-key")
//!     .with_base_url("https://myresource.openai.azure.com/openai/deployments/gpt4")
//!     .with_api_version("2024-02-15-preview");
//!
//! let request = AgentRequest::error("x + 1", "main.aura", 1, 1)
//!     .with_message("Variable 'x' not defined");
//!
//! let response = provider.send_request(request).await?;
//! ```
//!
//! ## Azure OpenAI Support
//!
//! Azure OpenAI uses a different URL structure and requires an api-version parameter.
//! Use `with_base_url` and `with_api_version` for Azure deployments.

use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use super::bridge::{AgentError, AgentProvider};
use super::prompts;
use super::request::AgentRequest;
use super::response::{AgentResponse, Patch, Suggestion};

/// Default model for OpenAI
const DEFAULT_MODEL: &str = "gpt-4";

/// Default base URL for OpenAI API
const DEFAULT_BASE_URL: &str = "https://api.openai.com/v1";

/// Default timeout for requests (30 seconds)
const DEFAULT_TIMEOUT_SECS: u64 = 30;

/// Default max tokens in response
const DEFAULT_MAX_TOKENS: usize = 4096;

/// Default temperature for generation
const DEFAULT_TEMPERATURE: f32 = 0.2;

/// OpenAI provider for self-healing
pub struct OpenAIProvider {
    /// API key for OpenAI or Azure
    api_key: String,
    /// Model to use
    model: String,
    /// Base URL for the API
    base_url: String,
    /// HTTP client
    client: reqwest::Client,
    /// Timeout for requests
    timeout: Duration,
    /// Max tokens in response
    max_tokens: usize,
    /// Temperature for generation
    temperature: f32,
    /// Organization ID (optional, for OpenAI)
    organization: Option<String>,
    /// API version (required for Azure OpenAI)
    api_version: Option<String>,
}

impl OpenAIProvider {
    /// Creates a new OpenAI provider
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            model: DEFAULT_MODEL.to_string(),
            base_url: DEFAULT_BASE_URL.to_string(),
            client: reqwest::Client::new(),
            timeout: Duration::from_secs(DEFAULT_TIMEOUT_SECS),
            max_tokens: DEFAULT_MAX_TOKENS,
            temperature: DEFAULT_TEMPERATURE,
            organization: None,
            api_version: None,
        }
    }

    /// Configures the model to use
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    /// Configures the base URL (for Azure or local endpoints)
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    /// Configures the timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Configures the max tokens
    pub fn with_max_tokens(mut self, max_tokens: usize) -> Self {
        self.max_tokens = max_tokens;
        self
    }

    /// Configures the temperature (0.0 = deterministic, 1.0 = creative)
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = temperature.clamp(0.0, 2.0);
        self
    }

    /// Configures the organization ID (for OpenAI)
    pub fn with_organization(mut self, org: impl Into<String>) -> Self {
        self.organization = Some(org.into());
        self
    }

    /// Configures the API version (required for Azure OpenAI)
    pub fn with_api_version(mut self, version: impl Into<String>) -> Self {
        self.api_version = Some(version.into());
        self
    }

    /// Formats an AgentRequest as a prompt for OpenAI
    fn format_prompt(&self, request: &AgentRequest) -> String {
        let mut prompt = String::new();

        // Add error information
        prompt.push_str("## Error in AURA\n\n");

        if let Some(msg) = &request.message {
            prompt.push_str(&format!("**Error message:** {}\n\n", msg));
        }

        // Add code
        prompt.push_str(&format!(
            "**Code:**\n```aura\n{}\n```\n\n",
            request.context.source
        ));

        // Add location
        prompt.push_str(&format!(
            "**Location:** {}:{}:{}\n\n",
            request.location.file, request.location.line, request.location.column
        ));

        // Add surrounding code if available
        if let Some(ref surrounding) = request.context.surrounding_code {
            if !surrounding.is_empty() {
                prompt.push_str(&format!(
                    "**Additional context:**\n```aura\n{}\n```\n\n",
                    surrounding
                ));
            }
        }

        // Add previous attempts if any
        if !request.previous_attempts.is_empty() {
            prompt.push_str("**Previous attempts that didn't work:**\n");
            for attempt in &request.previous_attempts {
                prompt.push_str(&format!("- `{}`\n", attempt));
            }
            prompt.push('\n');
        }

        prompt.push_str("Please analyze the error and provide a fix in the specified JSON format.");

        prompt
    }

    /// Parses the JSON response from OpenAI to AgentResponse
    fn parse_response(&self, json_str: &str) -> Result<AgentResponse, AgentError> {
        // Try to extract JSON from the text (model sometimes adds text before/after)
        let json_str = self.extract_json(json_str)?;

        let parsed: OpenAIHealingResponse = serde_json::from_str(&json_str).map_err(|e| {
            AgentError::SerializationError(format!("Error parsing response: {}", e))
        })?;

        // Convert to AgentResponse
        let response = match parsed.action.as_str() {
            "patch" => {
                let patch = parsed.patch.ok_or_else(|| {
                    AgentError::SerializationError(
                        "'patch' response missing patch field".to_string(),
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
                    AgentError::SerializationError("'generate' response missing code".to_string())
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
                    .unwrap_or_else(|| "Complex error".to_string()),
                &parsed.explanation,
            ),
            other => {
                return Err(AgentError::SerializationError(format!(
                    "Unknown action: {}",
                    other
                )));
            }
        };

        Ok(response)
    }

    /// Extracts JSON from text that may have additional content
    fn extract_json(&self, text: &str) -> Result<String, AgentError> {
        // Look for the start of a JSON object
        if let Some(start) = text.find('{') {
            // Find the corresponding closing brace
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
            "No valid JSON found in response".to_string(),
        ))
    }

    /// Calls the OpenAI API
    async fn call_api(&self, prompt: &str) -> Result<String, AgentError> {
        // Build URL with api-version for Azure
        let url = if let Some(ref version) = self.api_version {
            format!(
                "{}/chat/completions?api-version={}",
                self.base_url, version
            )
        } else {
            format!("{}/chat/completions", self.base_url)
        };

        let request_body = OpenAIApiRequest {
            model: self.model.clone(),
            messages: vec![
                OpenAIMessage {
                    role: "system".to_string(),
                    content: prompts::healing_system_prompt(),
                },
                OpenAIMessage {
                    role: "user".to_string(),
                    content: prompt.to_string(),
                },
            ],
            temperature: self.temperature,
            max_tokens: Some(self.max_tokens),
        };

        let mut request_builder = self
            .client
            .post(&url)
            .header("content-type", "application/json")
            .timeout(self.timeout)
            .json(&request_body);

        // Add authentication headers
        // Azure uses api-key header, OpenAI uses Bearer token
        if self.api_version.is_some() {
            // Azure OpenAI
            request_builder = request_builder.header("api-key", &self.api_key);
        } else {
            // Standard OpenAI
            request_builder =
                request_builder.header("authorization", format!("Bearer {}", self.api_key));

            // Add organization header if provided
            if let Some(ref org) = self.organization {
                request_builder = request_builder.header("openai-organization", org);
            }
        }

        let response = request_builder.send().await.map_err(|e| {
            if e.is_timeout() {
                AgentError::Timeout {
                    timeout_ms: self.timeout.as_millis() as u64,
                }
            } else if e.is_connect() {
                AgentError::ConnectionError(format!(
                    "Could not connect to OpenAI at {}: {}",
                    self.base_url, e
                ))
            } else {
                AgentError::ConnectionError(format!("Network error: {}", e))
            }
        })?;

        let status = response.status();

        if status.as_u16() == 429 {
            // Try to extract retry-after header
            let retry_after = response
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse::<u64>().ok())
                .map(|s| s * 1000); // Convert seconds to ms

            return Err(AgentError::RateLimited {
                retry_after_ms: retry_after.or(Some(60000)),
            });
        }

        if status.as_u16() == 401 {
            let error_text: String = response.text().await.unwrap_or_default();
            return Err(AgentError::AuthenticationError(format!(
                "Invalid API key or authentication failed: {}",
                error_text
            )));
        }

        if !status.is_success() {
            let error_text: String = response.text().await.unwrap_or_default();
            return Err(AgentError::InvalidResponse(format!(
                "API error ({}): {}",
                status.as_u16(),
                error_text
            )));
        }

        let api_response: OpenAIApiResponse = response.json().await.map_err(|e| {
            AgentError::SerializationError(format!("Error parsing API response: {}", e))
        })?;

        // Extract the text from the response
        api_response
            .choices
            .into_iter()
            .next()
            .map(|c| c.message.content)
            .ok_or_else(|| {
                AgentError::SerializationError("Response has no choices".to_string())
            })
    }
}

impl AgentProvider for OpenAIProvider {
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
        "openai"
    }

    fn is_available<'a>(&'a self) -> Pin<Box<dyn Future<Output = bool> + Send + 'a>> {
        Box::pin(async move {
            // Verify that we have an API key
            !self.api_key.is_empty()
        })
    }
}

// ============================================================================
// Structures for OpenAI API serialization
// ============================================================================

/// Request for the /chat/completions endpoint
#[derive(Serialize)]
struct OpenAIApiRequest {
    model: String,
    messages: Vec<OpenAIMessage>,
    temperature: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<usize>,
}

/// Message in OpenAI format
#[derive(Serialize, Deserialize)]
struct OpenAIMessage {
    role: String,
    content: String,
}

/// Response from the /chat/completions endpoint
#[derive(Deserialize)]
struct OpenAIApiResponse {
    choices: Vec<OpenAIChoice>,
    #[allow(dead_code)]
    model: Option<String>,
    #[allow(dead_code)]
    usage: Option<OpenAIUsage>,
}

/// A choice in the response
#[derive(Deserialize)]
struct OpenAIChoice {
    message: OpenAIMessage,
    #[allow(dead_code)]
    finish_reason: Option<String>,
}

/// Token usage information
#[derive(Deserialize)]
#[allow(dead_code)]
struct OpenAIUsage {
    prompt_tokens: usize,
    completion_tokens: usize,
    total_tokens: usize,
}

// ============================================================================
// Structures for healing response
// ============================================================================

#[derive(Deserialize)]
struct OpenAIHealingResponse {
    action: String,
    #[serde(default)]
    patch: Option<OpenAIPatch>,
    #[serde(default)]
    generated_code: Option<String>,
    #[serde(default)]
    suggestions: Option<Vec<OpenAISuggestion>>,
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
struct OpenAIPatch {
    old_code: String,
    new_code: String,
}

#[derive(Deserialize)]
struct OpenAISuggestion {
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

    fn create_test_provider() -> OpenAIProvider {
        OpenAIProvider::new("test-key")
    }

    #[test]
    fn test_default_configuration() {
        let provider = OpenAIProvider::new("test-key");
        assert_eq!(provider.base_url, "https://api.openai.com/v1");
        assert_eq!(provider.model, "gpt-4");
        assert_eq!(provider.timeout, Duration::from_secs(30));
        assert!(provider.organization.is_none());
        assert!(provider.api_version.is_none());
    }

    #[test]
    fn test_builder_pattern() {
        let provider = OpenAIProvider::new("test-key")
            .with_model("gpt-4-turbo")
            .with_base_url("https://custom.openai.com/v1")
            .with_timeout(Duration::from_secs(60))
            .with_max_tokens(8192)
            .with_temperature(0.5)
            .with_organization("org-123");

        assert_eq!(provider.model, "gpt-4-turbo");
        assert_eq!(provider.base_url, "https://custom.openai.com/v1");
        assert_eq!(provider.timeout, Duration::from_secs(60));
        assert_eq!(provider.max_tokens, 8192);
        assert_eq!(provider.temperature, 0.5);
        assert_eq!(provider.organization, Some("org-123".to_string()));
    }

    #[test]
    fn test_azure_configuration() {
        let provider = OpenAIProvider::new("azure-key")
            .with_base_url("https://myresource.openai.azure.com/openai/deployments/gpt4")
            .with_api_version("2024-02-15-preview");

        assert!(provider.base_url.contains("azure.com"));
        assert_eq!(
            provider.api_version,
            Some("2024-02-15-preview".to_string())
        );
    }

    #[test]
    fn test_temperature_clamping() {
        let provider = OpenAIProvider::new("test-key").with_temperature(5.0);
        assert_eq!(provider.temperature, 2.0); // Clamped to max

        let provider = OpenAIProvider::new("test-key").with_temperature(-1.0);
        assert_eq!(provider.temperature, 0.0); // Clamped to min
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
    fn test_extract_json_with_code_block() {
        let provider = create_test_provider();
        let text = r#"```json
{"action": "patch", "patch": {"old_code": "x", "new_code": "y"}, "explanation": "Fixed", "confidence": 0.9}
```"#;
        let result = provider.extract_json(text);
        assert!(result.is_ok());
    }

    #[test]
    fn test_format_prompt_basic() {
        let provider = create_test_provider();
        let request = AgentRequest::error("x + 1", "test.aura", 5, 1)
            .with_message("Variable 'x' not defined");

        let prompt = provider.format_prompt(&request);

        assert!(prompt.contains("Variable 'x' not defined"));
        assert!(prompt.contains("x + 1"));
        assert!(prompt.contains("test.aura"));
        assert!(prompt.contains("5:1")); // line:column
    }

    #[test]
    fn test_format_prompt_with_previous_attempts() {
        let provider = create_test_provider();
        let request = AgentRequest::error("x + 1", "test.aura", 1, 1)
            .with_message("Variable not defined")
            .with_previous_attempt("let x = nil")
            .with_previous_attempt("let x = undefined");

        let prompt = provider.format_prompt(&request);

        assert!(prompt.contains("Previous attempts"));
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
            "explanation": "Declare variable x",
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
            "explanation": "Fibonacci implementation",
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
                {"code": "let x = 0", "rationale": "Initialize to zero", "confidence": 0.8},
                {"code": "let x = nil", "rationale": "Use nil as initial value", "confidence": 0.6}
            ],
            "explanation": "Multiple options available",
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
                "What type should variable 'x' have?",
                "Is 'x' a number or a string?"
            ],
            "explanation": "Need more information",
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
            "escalation_reason": "Complex architecture problem",
            "explanation": "Requires manual analysis",
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
            "explanation": "Fix without patch",
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
        let provider = OpenAIProvider::new("test-key");
        assert_eq!(provider.name(), "openai");
    }

    #[test]
    fn test_system_prompt_used() {
        // Verify that the healing system prompt is available
        let prompt = prompts::healing_system_prompt();
        assert!(prompt.contains("AURA"));
        assert!(prompt.contains("action"));
    }
}
