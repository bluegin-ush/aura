//! Agent Bridge - Trait y proveedores para comunicación con agentes IA

use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use serde::{Deserialize, Serialize};

use super::request::AgentRequest;
use super::response::{AgentResponse, Patch, ResponseMetadata};

/// Errores del Agent Bridge
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AgentError {
    /// Error de conexión con el agente
    ConnectionError(String),
    /// Timeout esperando respuesta
    Timeout { timeout_ms: u64 },
    /// Error al serializar/deserializar
    SerializationError(String),
    /// Respuesta inválida del agente
    InvalidResponse(String),
    /// El agente rechazó la solicitud
    Rejected { reason: String },
    /// Límite de rate alcanzado
    RateLimited { retry_after_ms: Option<u64> },
    /// Error de autenticación
    AuthenticationError(String),
    /// Error interno del agente
    InternalError(String),
}

impl std::fmt::Display for AgentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ConnectionError(msg) => write!(f, "Error de conexión: {}", msg),
            Self::Timeout { timeout_ms } => write!(f, "Timeout después de {}ms", timeout_ms),
            Self::SerializationError(msg) => write!(f, "Error de serialización: {}", msg),
            Self::InvalidResponse(msg) => write!(f, "Respuesta inválida: {}", msg),
            Self::Rejected { reason } => write!(f, "Solicitud rechazada: {}", reason),
            Self::RateLimited { retry_after_ms } => {
                if let Some(ms) = retry_after_ms {
                    write!(f, "Rate limited. Reintentar en {}ms", ms)
                } else {
                    write!(f, "Rate limited")
                }
            }
            Self::AuthenticationError(msg) => write!(f, "Error de autenticación: {}", msg),
            Self::InternalError(msg) => write!(f, "Error interno: {}", msg),
        }
    }
}

impl std::error::Error for AgentError {}

/// Trait para proveedores de agentes IA
///
/// Implementa este trait para conectar AURA con diferentes agentes IA.
///
/// ## Ejemplo
///
/// ```ignore
/// struct MyProvider;
///
/// impl AgentProvider for MyProvider {
///     fn send_request<'a>(
///         &'a self,
///         request: AgentRequest,
///     ) -> Pin<Box<dyn Future<Output = Result<AgentResponse, AgentError>> + Send + 'a>> {
///         Box::pin(async move {
///             // Implementación...
///             Ok(AgentResponse::generate("...", "...", 0.9))
///         })
///     }
/// }
/// ```
pub trait AgentProvider: Send + Sync {
    /// Envía una solicitud al agente y espera su respuesta
    fn send_request<'a>(
        &'a self,
        request: AgentRequest,
    ) -> Pin<Box<dyn Future<Output = Result<AgentResponse, AgentError>> + Send + 'a>>;

    /// Nombre del proveedor
    fn name(&self) -> &str {
        "unknown"
    }

    /// Verifica si el proveedor está disponible
    fn is_available<'a>(&'a self) -> Pin<Box<dyn Future<Output = bool> + Send + 'a>> {
        Box::pin(async { true })
    }
}

/// Proveedor mock para pruebas
///
/// Genera respuestas predecibles basadas en el tipo de evento.
pub struct MockProvider {
    /// Nombre del proveedor
    name: String,
    /// Latencia simulada en ms
    latency_ms: u64,
    /// Contador de solicitudes
    request_count: AtomicU64,
    /// Si debe fallar
    should_fail: bool,
    /// Respuesta fija (si se configura)
    fixed_response: Option<AgentResponse>,
}

impl MockProvider {
    /// Crea un nuevo proveedor mock
    pub fn new() -> Self {
        Self {
            name: "mock".to_string(),
            latency_ms: 10,
            request_count: AtomicU64::new(0),
            should_fail: false,
            fixed_response: None,
        }
    }

    /// Configura la latencia simulada
    pub fn with_latency(mut self, ms: u64) -> Self {
        self.latency_ms = ms;
        self
    }

    /// Configura el proveedor para fallar
    pub fn failing(mut self) -> Self {
        self.should_fail = true;
        self
    }

    /// Configura una respuesta fija
    pub fn with_response(mut self, response: AgentResponse) -> Self {
        self.fixed_response = Some(response);
        self
    }

    /// Obtiene el número de solicitudes procesadas
    pub fn request_count(&self) -> u64 {
        self.request_count.load(Ordering::SeqCst)
    }

    /// Genera un fix inteligente basado en el error
    fn generate_smart_fix(&self, error_msg: &str, source: &str) -> String {
        // Detectar tipo de error y aplicar fix apropiado

        // Error: Variable no definida
        if error_msg.contains("Variable no definida:") || error_msg.contains("Undefined variable:") {
            if let Some(var_name) = error_msg.split(':').last().map(|s| s.trim()) {
                // Buscar dónde se usa la variable y agregar definición antes
                let lines: Vec<&str> = source.lines().collect();
                let mut result = Vec::new();
                let mut added = false;

                for line in &lines {
                    // Si la línea usa la variable y no hemos agregado la definición
                    if !added && line.contains(var_name) && !line.contains(&format!("{} =", var_name)) {
                        // Agregar definición con valor por defecto inteligente
                        let default_value = if var_name.contains("url") || var_name.contains("endpoint") || var_name.contains("api") {
                            "\"https://jsonplaceholder.typicode.com\""
                        } else if var_name.contains("token") || var_name.contains("key") {
                            "env.get(\"API_KEY\", \"demo-key\")"
                        } else if var_name == "name" || var_name == "s" || var_name == "str" {
                            "\"default\""
                        } else if var_name == "list" || var_name == "items" || var_name == "arr" {
                            "[]"
                        } else if var_name == "x" || var_name == "n" || var_name == "num" || var_name == "id" {
                            "1"
                        } else {
                            "nil"
                        };
                        result.push(format!("{} = {}", var_name, default_value));
                        added = true;
                    }
                    result.push(line.to_string());
                }
                return result.join("\n");
            }
        }

        // Error: Función no definida
        if error_msg.contains("Función no definida:") || error_msg.contains("Undefined function:") {
            if let Some(func_name) = error_msg.split(':').last().map(|s| s.trim()) {
                // Agregar stub de función al inicio
                let stub = format!("{}(x) = x  # TODO: implementar\n\n", func_name);
                return format!("{}{}", stub, source);
            }
        }

        // Error: División por cero - agregar guard
        if error_msg.contains("División por cero") || error_msg.contains("division by zero") {
            return source.replace("/ 0", "/ 1  # Fixed: was dividing by zero");
        }

        // Error: Tipo incorrecto
        if error_msg.contains("tipo") || error_msg.contains("type") {
            // Agregar comentario indicando el problema
            return format!("# TODO: Fix type error - {}\n{}", error_msg, source);
        }

        // Fallback: retornar código original con comentario
        format!("# Auto-fix applied\n{}", source)
    }

    /// Genera una explicación del fix
    fn generate_explanation(&self, error_msg: &str) -> String {
        if error_msg.contains("Variable no definida") || error_msg.contains("Undefined variable") {
            "La variable no estaba definida. Se agregó una definición con un valor por defecto.".to_string()
        } else if error_msg.contains("Función no definida") || error_msg.contains("Undefined function") {
            "La función no existía. Se agregó un stub que debe ser implementado.".to_string()
        } else if error_msg.contains("División por cero") || error_msg.contains("division by zero") {
            "Se detectó una división por cero. Se cambió el divisor a 1.".to_string()
        } else {
            format!("Error corregido: {}", error_msg)
        }
    }

    /// Genera una respuesta mock basada en el tipo de evento
    fn generate_mock_response(&self, request: &AgentRequest) -> AgentResponse {
        use super::request::EventType;

        let base_response = match request.event_type {
            EventType::Error => {
                // Analizar el error y generar un fix inteligente
                let error_msg = request.message.as_deref().unwrap_or("");
                let source = &request.context.source;

                let fixed_code = self.generate_smart_fix(error_msg, source);
                let explanation = self.generate_explanation(error_msg);

                let patch = Patch::new(source.clone(), fixed_code);
                AgentResponse::patch(patch, &explanation, 0.95)
            }
            EventType::Missing => {
                // Simular generación de código faltante
                AgentResponse::generate(
                    format!("// Generated stub\nfn placeholder() {{\n  todo!()\n}}"),
                    "Código generado como placeholder",
                    0.7,
                )
            }
            EventType::Performance => {
                // Simular sugerencia de optimización
                AgentResponse::suggest(
                    vec![super::response::Suggestion {
                        code: "// Optimized version".to_string(),
                        rationale: "Reducir complejidad algorítmica".to_string(),
                        confidence: 0.6,
                    }],
                    "Sugerencia de optimización",
                )
            }
            EventType::Expansion => {
                // Simular clarificación necesaria
                AgentResponse::clarify(
                    vec![
                        "Qué funcionalidad específica necesitas?".to_string(),
                        "Hay restricciones de rendimiento?".to_string(),
                    ],
                    "Necesito más detalles para expandir la funcionalidad",
                )
            }
        };

        base_response.with_metadata(ResponseMetadata {
            model_id: Some("mock-v1".to_string()),
            tokens_used: Some(100),
            processing_time_ms: Some(self.latency_ms),
            session_id: request.session_id.clone(),
        })
    }
}

impl Default for MockProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentProvider for MockProvider {
    fn send_request<'a>(
        &'a self,
        request: AgentRequest,
    ) -> Pin<Box<dyn Future<Output = Result<AgentResponse, AgentError>> + Send + 'a>> {
        Box::pin(async move {
            // Incrementar contador
            self.request_count.fetch_add(1, Ordering::SeqCst);

            // Simular latencia
            if self.latency_ms > 0 {
                tokio::time::sleep(Duration::from_millis(self.latency_ms)).await;
            }

            // Verificar si debe fallar
            if self.should_fail {
                return Err(AgentError::InternalError(
                    "Mock configurado para fallar".to_string(),
                ));
            }

            // Retornar respuesta fija o generada
            Ok(self.fixed_response.clone().unwrap_or_else(|| {
                self.generate_mock_response(&request)
            }))
        })
    }

    fn name(&self) -> &str {
        &self.name
    }
}

/// Placeholder para el proveedor de Claude API
///
/// Esta implementación se completará cuando se integre con la API real de Claude.
#[cfg(feature = "claude-api")]
pub struct ClaudeProvider {
    /// API key
    api_key: String,
    /// URL base de la API
    base_url: String,
    /// Modelo a usar
    model: String,
    /// Cliente HTTP (placeholder)
    _client: (),
}

#[cfg(feature = "claude-api")]
impl ClaudeProvider {
    /// Crea un nuevo proveedor de Claude
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: "https://api.anthropic.com/v1".to_string(),
            model: "claude-sonnet-4-20250514".to_string(),
            _client: (),
        }
    }

    /// Configura el modelo a usar
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    /// Configura la URL base
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }
}

#[cfg(feature = "claude-api")]
impl AgentProvider for ClaudeProvider {
    fn send_request<'a>(
        &'a self,
        _request: AgentRequest,
    ) -> Pin<Box<dyn Future<Output = Result<AgentResponse, AgentError>> + Send + 'a>> {
        Box::pin(async move {
            // TODO: Implementar llamada HTTP real a la API de Claude
            //
            // Pasos:
            // 1. Serializar AgentRequest a formato de mensaje de Claude
            // 2. Construir el prompt del sistema para AURA
            // 3. Hacer llamada HTTP POST a /messages
            // 4. Parsear la respuesta y convertir a AgentResponse
            //
            // Por ahora, retornamos un error indicando que no está implementado
            Err(AgentError::InternalError(
                "Claude API no implementado aún. Usa MockProvider para pruebas.".to_string(),
            ))
        })
    }

    fn name(&self) -> &str {
        "claude"
    }

    fn is_available<'a>(&'a self) -> Pin<Box<dyn Future<Output = bool> + Send + 'a>> {
        Box::pin(async move {
            // TODO: Verificar conectividad con la API
            !self.api_key.is_empty()
        })
    }
}

/// Selector de proveedor basado en disponibilidad
pub struct ProviderSelector {
    providers: Vec<Box<dyn AgentProvider>>,
}

impl ProviderSelector {
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
        }
    }

    pub fn add_provider(mut self, provider: impl AgentProvider + 'static) -> Self {
        self.providers.push(Box::new(provider));
        self
    }

    /// Obtiene el primer proveedor disponible
    pub async fn get_available(&self) -> Option<&dyn AgentProvider> {
        for provider in &self.providers {
            if provider.is_available().await {
                return Some(provider.as_ref());
            }
        }
        None
    }
}

impl Default for ProviderSelector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::{EventType, AgentRequest, Action};

    #[tokio::test]
    async fn test_mock_provider() {
        let provider = MockProvider::new();

        let request = AgentRequest::error("x + 1", "test.aura", 1, 1)
            .with_message("Variable 'x' no definida");

        let response = provider.send_request(request).await.unwrap();

        assert_eq!(response.action, Action::Patch);
        assert!(response.patch.is_some());
        assert!(response.confidence > 0.0);
    }

    #[tokio::test]
    async fn test_mock_provider_failing() {
        let provider = MockProvider::new().failing();

        let request = AgentRequest::new(EventType::Error);
        let result = provider.send_request(request).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_mock_provider_fixed_response() {
        let fixed = AgentResponse::escalate(
            "Demasiado complejo",
            "No puedo resolver esto",
        );

        let provider = MockProvider::new().with_response(fixed.clone());

        let request = AgentRequest::new(EventType::Error);
        let response = provider.send_request(request).await.unwrap();

        assert_eq!(response.action, Action::Escalate);
    }

    #[tokio::test]
    async fn test_mock_provider_request_count() {
        let provider = MockProvider::new().with_latency(0);

        assert_eq!(provider.request_count(), 0);

        provider.send_request(AgentRequest::new(EventType::Error)).await.unwrap();
        provider.send_request(AgentRequest::new(EventType::Missing)).await.unwrap();

        assert_eq!(provider.request_count(), 2);
    }

    #[tokio::test]
    async fn test_provider_selector() {
        let selector = ProviderSelector::new()
            .add_provider(MockProvider::new().failing())
            .add_provider(MockProvider::new());

        // El primer proveedor está disponible aunque falle en las solicitudes
        let available = selector.get_available().await;
        assert!(available.is_some());
    }

    #[test]
    fn test_agent_error_display() {
        let error = AgentError::Timeout { timeout_ms: 5000 };
        assert_eq!(error.to_string(), "Timeout después de 5000ms");

        let error = AgentError::RateLimited { retry_after_ms: Some(1000) };
        assert_eq!(error.to_string(), "Rate limited. Reintentar en 1000ms");
    }
}
