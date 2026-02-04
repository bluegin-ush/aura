//! Self-Healing - Auto-reparacion de errores en runtime
//!
//! Cuando la VM encuentra un error, este modulo:
//! 1. Empaqueta el contexto del error
//! 2. Envia la solicitud al AgentProvider
//! 3. Aplica el fix si es seguro
//! 4. Continua la ejecucion
//!
//! ## Ejemplo de uso
//!
//! ```ignore
//! use aura::agent::{HealingEngine, HealingContext, MockProvider};
//! use aura::vm::RuntimeError;
//!
//! let provider = MockProvider::new();
//! let engine = HealingEngine::new(provider)
//!     .with_auto_apply(true)
//!     .with_max_attempts(3);
//!
//! let error = RuntimeError::new("Variable no definida: x");
//! let context = HealingContext {
//!     source_code: "fn main() { x + 1 }".to_string(),
//!     file_name: "main.aura".to_string(),
//!     line: 1,
//!     column: 15,
//!     surrounding_code: None,
//! };
//!
//! let result = engine.heal_error(&error, &context).await?;
//! ```

use std::fmt;
use std::path::PathBuf;

use super::{AgentRequest, AgentResponse, AgentProvider, AgentError, Action, Context};
use super::snapshot::{SnapshotReason, SnapshotId};
use super::undo::{UndoManager, HealingAction, VerificationResult};
use super::response::Patch;
use crate::vm::RuntimeError;

/// Umbral minimo de confianza para aplicar fixes automaticamente
const DEFAULT_CONFIDENCE_THRESHOLD: f32 = 0.8;

/// Numero maximo de intentos de reparacion por defecto
const DEFAULT_MAX_ATTEMPTS: usize = 3;

/// Motor de auto-reparacion de errores
///
/// Coordina la comunicacion entre la VM y el agente IA para
/// intentar reparar errores de runtime automaticamente.
pub struct HealingEngine<P: AgentProvider> {
    provider: P,
    /// Si true, aplica fixes automaticamente cuando la confianza es alta
    auto_apply: bool,
    /// Numero maximo de intentos de reparacion
    max_attempts: usize,
    /// Umbral de confianza para aplicar fixes automaticamente
    confidence_threshold: f32,
    /// Historial de intentos fallidos (para evitar repetir soluciones)
    previous_attempts: Vec<String>,
}

impl<P: AgentProvider> HealingEngine<P> {
    /// Crea un nuevo motor de auto-reparacion
    pub fn new(provider: P) -> Self {
        Self {
            provider,
            auto_apply: false,
            max_attempts: DEFAULT_MAX_ATTEMPTS,
            confidence_threshold: DEFAULT_CONFIDENCE_THRESHOLD,
            previous_attempts: Vec::new(),
        }
    }

    /// Configura si se aplican fixes automaticamente
    pub fn with_auto_apply(mut self, auto: bool) -> Self {
        self.auto_apply = auto;
        self
    }

    /// Configura el numero maximo de intentos
    pub fn with_max_attempts(mut self, attempts: usize) -> Self {
        self.max_attempts = attempts;
        self
    }

    /// Configura el umbral de confianza para auto-aplicar
    pub fn with_confidence_threshold(mut self, threshold: f32) -> Self {
        self.confidence_threshold = threshold.clamp(0.0, 1.0);
        self
    }

    /// Limpia el historial de intentos previos
    pub fn clear_history(&mut self) {
        self.previous_attempts.clear();
    }

    /// Intenta reparar un error de runtime
    ///
    /// Empaqueta el contexto del error, lo envia al agente, y retorna
    /// el resultado de la reparacion.
    pub async fn heal_error(
        &mut self,
        error: &RuntimeError,
        context: &HealingContext,
    ) -> Result<HealingResult, HealingError> {
        // Verificar limite de intentos
        if self.previous_attempts.len() >= self.max_attempts {
            return Err(HealingError::MaxAttemptsReached);
        }

        // Construir la solicitud al agente
        let request = self.build_request(error, context);

        // Enviar solicitud al agente
        let response = self.provider
            .send_request(request)
            .await
            .map_err(HealingError::ProviderError)?;

        // Procesar la respuesta
        let result = self.process_response(response, context)?;

        // Registrar intento si fue un fix
        if let HealingResult::Fixed { ref patch, .. } = result {
            self.previous_attempts.push(patch.clone());
        }

        Ok(result)
    }

    /// Intenta reparar un error multiples veces hasta conseguirlo o agotar intentos
    pub async fn heal_with_retry(
        &mut self,
        error: &RuntimeError,
        context: &HealingContext,
    ) -> Result<HealingResult, HealingError> {
        let mut last_error = None;

        for attempt in 0..self.max_attempts {
            match self.heal_error(error, context).await {
                Ok(result) => {
                    // Si obtuvimos un fix con alta confianza, retornarlo
                    if matches!(&result, HealingResult::Fixed { .. }) {
                        return Ok(result);
                    }
                    // Si necesita humano o no se puede arreglar, retornar inmediatamente
                    if matches!(&result, HealingResult::NeedsHuman { .. } | HealingResult::CannotFix { .. }) {
                        return Ok(result);
                    }
                    // Si son sugerencias, retornarlas despues del ultimo intento
                    if attempt == self.max_attempts - 1 {
                        return Ok(result);
                    }
                }
                Err(e) => {
                    last_error = Some(e);
                    // Si es MaxAttemptsReached, no seguir intentando
                    if matches!(last_error, Some(HealingError::MaxAttemptsReached)) {
                        break;
                    }
                }
            }
        }

        Err(last_error.unwrap_or(HealingError::MaxAttemptsReached))
    }

    /// Construye una solicitud al agente basada en el error y contexto
    fn build_request(&self, error: &RuntimeError, context: &HealingContext) -> AgentRequest {
        let agent_context = Context::new(&context.source_code)
            .with_surrounding(context.surrounding_code.clone().unwrap_or_default());

        let mut request = AgentRequest::error(
            &context.source_code,
            &context.file_name,
            context.line,
            context.column,
        )
        .with_full_context(agent_context)
        .with_message(&error.message);

        // Agregar intentos previos para evitar repetir soluciones
        for attempt in &self.previous_attempts {
            request = request.with_previous_attempt(attempt);
        }

        request
    }

    /// Procesa la respuesta del agente y la convierte en HealingResult
    fn process_response(
        &self,
        response: AgentResponse,
        _context: &HealingContext,
    ) -> Result<HealingResult, HealingError> {
        match response.action {
            Action::Patch => {
                let patch = response.patch.ok_or_else(|| {
                    HealingError::InvalidResponse("Respuesta Patch sin patch incluido".to_string())
                })?;

                // Verificar si debemos auto-aplicar
                let should_auto_apply = self.auto_apply
                    && response.confidence >= self.confidence_threshold;

                if should_auto_apply {
                    Ok(HealingResult::Fixed {
                        patch: patch.new_code,
                        explanation: response.explanation,
                    })
                } else {
                    // Retornar como sugerencia si no auto-aplicamos
                    Ok(HealingResult::Suggested {
                        suggestions: vec![
                            format!("Patch sugerido (confianza: {:.0}%): {}",
                                response.confidence * 100.0,
                                patch.new_code
                            )
                        ],
                    })
                }
            }

            Action::Generate => {
                let code = response.generated_code.ok_or_else(|| {
                    HealingError::InvalidResponse("Respuesta Generate sin codigo".to_string())
                })?;

                let should_auto_apply = self.auto_apply
                    && response.confidence >= self.confidence_threshold;

                if should_auto_apply {
                    Ok(HealingResult::Fixed {
                        patch: code,
                        explanation: response.explanation,
                    })
                } else {
                    Ok(HealingResult::Suggested {
                        suggestions: vec![
                            format!("Codigo generado (confianza: {:.0}%): {}",
                                response.confidence * 100.0,
                                code
                            )
                        ],
                    })
                }
            }

            Action::Suggest => {
                let suggestions: Vec<String> = response.suggestions
                    .into_iter()
                    .map(|s| format!("{} (confianza: {:.0}%)", s.code, s.confidence * 100.0))
                    .collect();

                if suggestions.is_empty() {
                    Ok(HealingResult::CannotFix {
                        reason: response.explanation,
                    })
                } else {
                    Ok(HealingResult::Suggested { suggestions })
                }
            }

            Action::Clarify => {
                let questions = response.questions.join("\n- ");
                Ok(HealingResult::NeedsHuman {
                    reason: format!(
                        "El agente necesita mas informacion:\n- {}",
                        if questions.is_empty() {
                            response.explanation
                        } else {
                            questions
                        }
                    ),
                })
            }

            Action::Escalate => {
                Ok(HealingResult::NeedsHuman {
                    reason: response.escalation_reason.unwrap_or(response.explanation),
                })
            }
        }
    }

    /// Verifica si el motor esta habilitado para auto-aplicar fixes
    pub fn is_auto_apply_enabled(&self) -> bool {
        self.auto_apply
    }

    /// Obtiene el umbral de confianza actual
    pub fn confidence_threshold(&self) -> f32 {
        self.confidence_threshold
    }

    /// Obtiene el numero de intentos previos
    pub fn attempts_count(&self) -> usize {
        self.previous_attempts.len()
    }

    /// Obtiene el numero maximo de intentos
    pub fn max_attempts(&self) -> usize {
        self.max_attempts
    }

    /// Intenta reparar un error de forma segura con snapshots
    ///
    /// Esta version crea un snapshot antes de aplicar cualquier fix,
    /// permitiendo revertir cambios si la reparacion falla.
    pub async fn heal_error_safe(
        &mut self,
        error: &RuntimeError,
        context: &HealingContext,
        undo_manager: &mut UndoManager,
        file_content: &str,
    ) -> Result<SafeHealingResult, HealingError> {
        // 1. Crear snapshot antes de cualquier cambio
        let file_path = PathBuf::from(&context.file_name);
        let snapshot_id = undo_manager
            .create_snapshot_with_files(
                SnapshotReason::BeforeHeal {
                    error_id: error.message.clone(),
                },
                vec![(file_path.clone(), file_content.to_string())],
            )
            .map_err(|e| HealingError::SnapshotError(e.to_string()))?;

        // 2. Intentar healing normal
        let result = self.heal_error(error, context).await?;

        // 3. Si se obtuvo un fix, registrar la accion
        if let HealingResult::Fixed { ref patch, ref explanation } = result {
            // Crear el objeto Patch para el historial
            let patch_obj = Patch::new(&context.source_code, patch)
                .with_location(&context.file_name, context.line, context.line);

            // Registrar la accion en el historial
            let action = HealingAction::new(
                snapshot_id.clone(),
                patch_obj,
                self.confidence_threshold, // Sabemos que paso el threshold
                file_path,
            );

            undo_manager.record_action(action);

            return Ok(SafeHealingResult::Fixed {
                snapshot_id,
                patch: patch.clone(),
                explanation: explanation.clone(),
            });
        }

        // Si no se aplico fix, devolver el resultado normal sin registrar accion
        Ok(SafeHealingResult::from_healing_result(result, snapshot_id))
    }

    /// Intenta reparar y verificar, revirtiendo si la verificacion falla
    pub async fn heal_and_verify<V: FnOnce(&str) -> VerificationResult>(
        &mut self,
        error: &RuntimeError,
        context: &HealingContext,
        undo_manager: &mut UndoManager,
        file_content: &str,
        verifier: V,
    ) -> Result<SafeHealingResult, HealingError> {
        let result = self.heal_error_safe(error, context, undo_manager, file_content).await?;

        if let SafeHealingResult::Fixed { ref snapshot_id, ref patch, .. } = result {
            // Verificar el fix
            let verification = verifier(patch);

            match verification {
                VerificationResult::Failure { ref error, .. } => {
                    // Preparar para revertir
                    if undo_manager.can_undo() {
                        // El caller debe aplicar el undo manualmente
                        return Ok(SafeHealingResult::VerificationFailed {
                            snapshot_id: snapshot_id.clone(),
                            patch: patch.clone(),
                            error: error.clone(),
                        });
                    }
                }
                _ => {
                    // Verificacion exitosa, actualizar el historial
                    // (La accion ya fue registrada en heal_error_safe)
                }
            }
        }

        Ok(result)
    }
}

/// Contexto para la reparacion de errores
///
/// Contiene toda la informacion necesaria para que el agente
/// pueda entender y reparar el error.
#[derive(Debug, Clone)]
pub struct HealingContext {
    /// Codigo fuente donde ocurrio el error
    pub source_code: String,
    /// Nombre del archivo
    pub file_name: String,
    /// Linea donde ocurrio el error (1-indexed)
    pub line: usize,
    /// Columna donde ocurrio el error (1-indexed)
    pub column: usize,
    /// Codigo circundante para mas contexto
    pub surrounding_code: Option<String>,
}

impl HealingContext {
    /// Crea un nuevo contexto de reparacion
    pub fn new(source_code: impl Into<String>, file_name: impl Into<String>, line: usize, column: usize) -> Self {
        Self {
            source_code: source_code.into(),
            file_name: file_name.into(),
            line,
            column,
            surrounding_code: None,
        }
    }

    /// Agrega codigo circundante
    pub fn with_surrounding(mut self, code: impl Into<String>) -> Self {
        self.surrounding_code = Some(code.into());
        self
    }

    /// Crea contexto desde un RuntimeError (cuando tenemos informacion de ubicacion)
    pub fn from_error(error: &RuntimeError, source: impl Into<String>, file: impl Into<String>) -> Self {
        // Por ahora RuntimeError solo tiene message, asi que usamos defaults
        Self {
            source_code: source.into(),
            file_name: file.into(),
            line: 1,
            column: 1,
            surrounding_code: None,
        }
    }
}

/// Resultado de un intento de reparacion
#[derive(Debug, Clone)]
pub enum HealingResult {
    /// El error fue reparado exitosamente
    Fixed {
        /// El codigo corregido o parche a aplicar
        patch: String,
        /// Explicacion de lo que se cambio
        explanation: String,
    },
    /// Se tienen sugerencias pero no se aplicaron automaticamente
    Suggested {
        /// Lista de sugerencias de codigo
        suggestions: Vec<String>,
    },
    /// Se necesita intervencion humana
    NeedsHuman {
        /// Razon por la que se necesita un humano
        reason: String,
    },
    /// No se puede reparar el error
    CannotFix {
        /// Razon por la que no se puede reparar
        reason: String,
    },
}

impl HealingResult {
    /// Verifica si el resultado es un fix exitoso
    pub fn is_fixed(&self) -> bool {
        matches!(self, HealingResult::Fixed { .. })
    }

    /// Verifica si hay sugerencias disponibles
    pub fn has_suggestions(&self) -> bool {
        matches!(self, HealingResult::Suggested { suggestions } if !suggestions.is_empty())
    }

    /// Verifica si se necesita intervencion humana
    pub fn needs_human(&self) -> bool {
        matches!(self, HealingResult::NeedsHuman { .. })
    }

    /// Obtiene el patch si esta disponible
    pub fn get_patch(&self) -> Option<&str> {
        match self {
            HealingResult::Fixed { patch, .. } => Some(patch),
            _ => None,
        }
    }

    /// Obtiene las sugerencias si estan disponibles
    pub fn get_suggestions(&self) -> Option<&[String]> {
        match self {
            HealingResult::Suggested { suggestions } => Some(suggestions),
            _ => None,
        }
    }
}

/// Resultado de healing seguro con snapshots
#[derive(Debug, Clone)]
pub enum SafeHealingResult {
    /// Fix aplicado exitosamente con snapshot creado
    Fixed {
        /// ID del snapshot creado antes del fix
        snapshot_id: SnapshotId,
        /// El patch aplicado
        patch: String,
        /// Explicacion del fix
        explanation: String,
    },
    /// Sugerencias disponibles (no se modifico nada)
    Suggested {
        /// ID del snapshot (puede no tener cambios)
        snapshot_id: SnapshotId,
        /// Sugerencias de codigo
        suggestions: Vec<String>,
    },
    /// Se necesita intervencion humana
    NeedsHuman {
        /// ID del snapshot
        snapshot_id: SnapshotId,
        /// Razon
        reason: String,
    },
    /// No se puede reparar
    CannotFix {
        /// ID del snapshot
        snapshot_id: SnapshotId,
        /// Razon
        reason: String,
    },
    /// Fix aplicado pero verificacion fallo (se debe revertir)
    VerificationFailed {
        /// ID del snapshot para revertir
        snapshot_id: SnapshotId,
        /// El patch que fallo
        patch: String,
        /// Error de verificacion
        error: String,
    },
}

impl SafeHealingResult {
    /// Convierte un HealingResult normal a SafeHealingResult
    pub fn from_healing_result(result: HealingResult, snapshot_id: SnapshotId) -> Self {
        match result {
            HealingResult::Fixed { patch, explanation } => Self::Fixed {
                snapshot_id,
                patch,
                explanation,
            },
            HealingResult::Suggested { suggestions } => Self::Suggested {
                snapshot_id,
                suggestions,
            },
            HealingResult::NeedsHuman { reason } => Self::NeedsHuman {
                snapshot_id,
                reason,
            },
            HealingResult::CannotFix { reason } => Self::CannotFix {
                snapshot_id,
                reason,
            },
        }
    }

    /// Verifica si el resultado es un fix exitoso
    pub fn is_fixed(&self) -> bool {
        matches!(self, Self::Fixed { .. })
    }

    /// Verifica si la verificacion fallo
    pub fn verification_failed(&self) -> bool {
        matches!(self, Self::VerificationFailed { .. })
    }

    /// Obtiene el snapshot ID
    pub fn snapshot_id(&self) -> &SnapshotId {
        match self {
            Self::Fixed { snapshot_id, .. } |
            Self::Suggested { snapshot_id, .. } |
            Self::NeedsHuman { snapshot_id, .. } |
            Self::CannotFix { snapshot_id, .. } |
            Self::VerificationFailed { snapshot_id, .. } => snapshot_id,
        }
    }
}

/// Errores del motor de auto-reparacion
#[derive(Debug, Clone)]
pub enum HealingError {
    /// Error del proveedor de agentes
    ProviderError(AgentError),
    /// Respuesta invalida del agente
    InvalidResponse(String),
    /// Se alcanzo el maximo de intentos
    MaxAttemptsReached,
    /// Error del sistema de snapshots
    SnapshotError(String),
}

impl fmt::Display for HealingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ProviderError(e) => write!(f, "Error del proveedor: {}", e),
            Self::InvalidResponse(msg) => write!(f, "Respuesta invalida: {}", msg),
            Self::MaxAttemptsReached => write!(f, "Se alcanzo el maximo de intentos de reparacion"),
            Self::SnapshotError(msg) => write!(f, "Error de snapshot: {}", msg),
        }
    }
}

impl std::error::Error for HealingError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::ProviderError(e) => Some(e),
            _ => None,
        }
    }
}

impl From<AgentError> for HealingError {
    fn from(error: AgentError) -> Self {
        HealingError::ProviderError(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::{MockProvider, Patch};

    #[tokio::test]
    async fn test_heal_error_basic() {
        let provider = MockProvider::new().with_latency(0);
        let mut engine = HealingEngine::new(provider);

        let error = RuntimeError::new("Variable no definida: x");
        let context = HealingContext::new(
            "x + 1",
            "test.aura",
            1,
            1,
        );

        let result = engine.heal_error(&error, &context).await;
        assert!(result.is_ok());

        // MockProvider retorna Patch para errores, pero sin auto_apply retorna Suggested
        let healing_result = result.unwrap();
        assert!(healing_result.has_suggestions() || healing_result.is_fixed());
    }

    #[tokio::test]
    async fn test_heal_error_with_auto_apply() {
        let provider = MockProvider::new().with_latency(0);
        let mut engine = HealingEngine::new(provider)
            .with_auto_apply(true)
            .with_confidence_threshold(0.5); // MockProvider retorna 0.85

        let error = RuntimeError::new("Variable no definida: x");
        let context = HealingContext::new("x + 1", "test.aura", 1, 1);

        let result = engine.heal_error(&error, &context).await.unwrap();

        // Con auto_apply y confianza alta, deberia ser Fixed
        assert!(result.is_fixed());
        assert!(result.get_patch().is_some());
    }

    #[tokio::test]
    async fn test_heal_error_low_confidence() {
        // Crear respuesta con baja confianza
        let response = AgentResponse::patch(
            Patch::new("old", "new"),
            "Fix de prueba",
            0.3, // Baja confianza
        );

        let provider = MockProvider::new()
            .with_latency(0)
            .with_response(response);

        let mut engine = HealingEngine::new(provider)
            .with_auto_apply(true)
            .with_confidence_threshold(0.8);

        let error = RuntimeError::new("Error de prueba");
        let context = HealingContext::new("codigo", "test.aura", 1, 1);

        let result = engine.heal_error(&error, &context).await.unwrap();

        // Con baja confianza, deberia ser Suggested, no Fixed
        assert!(result.has_suggestions());
        assert!(!result.is_fixed());
    }

    #[tokio::test]
    async fn test_heal_error_escalate() {
        let response = AgentResponse::escalate(
            "Demasiado complejo",
            "Este error requiere intervencion humana",
        );

        let provider = MockProvider::new()
            .with_latency(0)
            .with_response(response);

        let mut engine = HealingEngine::new(provider);

        let error = RuntimeError::new("Error complejo");
        let context = HealingContext::new("codigo complejo", "test.aura", 1, 1);

        let result = engine.heal_error(&error, &context).await.unwrap();

        assert!(result.needs_human());
    }

    #[tokio::test]
    async fn test_heal_error_clarify() {
        let response = AgentResponse::clarify(
            vec!["Cual es el tipo de x?".to_string()],
            "Necesito mas informacion",
        );

        let provider = MockProvider::new()
            .with_latency(0)
            .with_response(response);

        let mut engine = HealingEngine::new(provider);

        let error = RuntimeError::new("Error ambiguo");
        let context = HealingContext::new("x + y", "test.aura", 1, 1);

        let result = engine.heal_error(&error, &context).await.unwrap();

        assert!(result.needs_human());
    }

    #[tokio::test]
    async fn test_heal_error_max_attempts() {
        let provider = MockProvider::new().with_latency(0);
        let mut engine = HealingEngine::new(provider)
            .with_max_attempts(2)
            .with_auto_apply(true)
            .with_confidence_threshold(0.5); // MockProvider returns 0.85 confidence

        let error = RuntimeError::new("Error");
        let context = HealingContext::new("codigo", "test.aura", 1, 1);

        // Primer intento - retorna Fixed y registra el intento
        let _ = engine.heal_error(&error, &context).await;
        assert_eq!(engine.attempts_count(), 1);

        // Segundo intento
        let _ = engine.heal_error(&error, &context).await;
        assert_eq!(engine.attempts_count(), 2);

        // Tercer intento deberia fallar
        let result = engine.heal_error(&error, &context).await;
        assert!(matches!(result, Err(HealingError::MaxAttemptsReached)));
    }

    #[tokio::test]
    async fn test_heal_error_provider_failure() {
        let provider = MockProvider::new()
            .with_latency(0)
            .failing();

        let mut engine = HealingEngine::new(provider);

        let error = RuntimeError::new("Error");
        let context = HealingContext::new("codigo", "test.aura", 1, 1);

        let result = engine.heal_error(&error, &context).await;

        assert!(matches!(result, Err(HealingError::ProviderError(_))));
    }

    #[tokio::test]
    async fn test_clear_history() {
        let provider = MockProvider::new().with_latency(0);
        let mut engine = HealingEngine::new(provider)
            .with_auto_apply(true)
            .with_confidence_threshold(0.5);

        let error = RuntimeError::new("Error");
        let context = HealingContext::new("codigo", "test.aura", 1, 1);

        // Hacer un intento
        let _ = engine.heal_error(&error, &context).await;
        assert_eq!(engine.attempts_count(), 1);

        // Limpiar historial
        engine.clear_history();
        assert_eq!(engine.attempts_count(), 0);
    }

    #[tokio::test]
    async fn test_healing_context_builder() {
        let context = HealingContext::new("x + 1", "test.aura", 5, 10)
            .with_surrounding("fn main() {\n  x + 1\n}");

        assert_eq!(context.source_code, "x + 1");
        assert_eq!(context.file_name, "test.aura");
        assert_eq!(context.line, 5);
        assert_eq!(context.column, 10);
        assert!(context.surrounding_code.is_some());
    }

    #[test]
    fn test_healing_result_helpers() {
        let fixed = HealingResult::Fixed {
            patch: "new code".to_string(),
            explanation: "Fixed!".to_string(),
        };
        assert!(fixed.is_fixed());
        assert!(!fixed.needs_human());
        assert_eq!(fixed.get_patch(), Some("new code"));

        let suggested = HealingResult::Suggested {
            suggestions: vec!["try this".to_string()],
        };
        assert!(!suggested.is_fixed());
        assert!(suggested.has_suggestions());
        assert!(suggested.get_suggestions().is_some());

        let needs_human = HealingResult::NeedsHuman {
            reason: "too complex".to_string(),
        };
        assert!(needs_human.needs_human());
        assert!(!needs_human.is_fixed());

        let cannot_fix = HealingResult::CannotFix {
            reason: "unsupported".to_string(),
        };
        assert!(!cannot_fix.is_fixed());
        assert!(!cannot_fix.needs_human());
    }

    #[test]
    fn test_healing_error_display() {
        let provider_error = HealingError::ProviderError(
            AgentError::Timeout { timeout_ms: 5000 }
        );
        assert!(provider_error.to_string().contains("proveedor"));

        let invalid = HealingError::InvalidResponse("bad data".to_string());
        assert!(invalid.to_string().contains("invalida"));

        let max_attempts = HealingError::MaxAttemptsReached;
        assert!(max_attempts.to_string().contains("maximo"));
    }

    #[test]
    fn test_healing_engine_config() {
        let provider = MockProvider::new();
        let engine = HealingEngine::new(provider)
            .with_auto_apply(true)
            .with_max_attempts(5)
            .with_confidence_threshold(0.9);

        assert!(engine.is_auto_apply_enabled());
        assert_eq!(engine.max_attempts(), 5);
        assert_eq!(engine.confidence_threshold(), 0.9);
    }
}
