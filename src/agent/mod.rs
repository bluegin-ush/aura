//! Agent Bridge - Protocolo de comunicación entre el runtime y agentes IA
//!
//! Este módulo define el protocolo para que el runtime de AURA pueda
//! comunicarse con agentes IA externos (como Claude) para:
//!
//! - Resolver errores automáticamente
//! - Generar código faltante
//! - Optimizar rendimiento
//! - Expandir funcionalidad
//!
//! ## Arquitectura
//!
//! ```text
//! ┌─────────────┐     AgentRequest      ┌─────────────┐
//! │   Runtime   │ ──────────────────────▶│   Agente    │
//! │    AURA     │                        │   (Claude)  │
//! │             │ ◀──────────────────────│             │
//! └─────────────┘     AgentResponse      └─────────────┘
//! ```
//!
//! ## Self-Healing
//!
//! El módulo `healing` proporciona auto-reparación de errores:
//!
//! ```ignore
//! use aura::agent::{HealingEngine, HealingContext, MockProvider};
//! use aura::vm::RuntimeError;
//!
//! let provider = MockProvider::new();
//! let mut engine = HealingEngine::new(provider)
//!     .with_auto_apply(true);
//!
//! let error = RuntimeError::new("Variable no definida: x");
//! let context = HealingContext::new("x + 1", "main.aura", 1, 1);
//!
//! let result = engine.heal_error(&error, &context).await?;
//! if result.is_fixed() {
//!     println!("Reparado: {:?}", result.get_patch());
//! }
//! ```
//!
//! ## Ejemplo de uso
//!
//! ```ignore
//! use aura::agent::{AgentRequest, EventType, AgentProvider, MockProvider};
//!
//! let provider = MockProvider::new();
//! let request = AgentRequest::new(EventType::Error)
//!     .with_context("fn main() { x + 1 }")
//!     .with_location("main.aura", 1, 15);
//!
//! let response = provider.send_request(request).await?;
//! println!("Acción: {:?}", response.action);
//! ```

mod request;
mod response;
mod bridge;
mod healing;
mod snapshot;
mod undo;
pub mod prompts;
#[cfg(feature = "claude-api")]
mod claude;
#[cfg(feature = "ollama")]
mod ollama;

pub use request::{AgentRequest, EventType, Context, SourceLocation, Constraints};
pub use response::{AgentResponse, Action, Patch, Suggestion};
pub use bridge::{AgentProvider, AgentError, MockProvider};
pub use healing::{HealingEngine, HealingContext, HealingResult, HealingError, SafeHealingResult};
pub use snapshot::{Snapshot, SnapshotId, SnapshotManager, SnapshotReason, SnapshotError, FileSnapshot, SnapshotSummary, RestoreResult};
pub use undo::{UndoManager, UndoError, HealingAction, VerificationResult, UndoResult, RedoResult};

#[cfg(feature = "claude-api")]
pub use claude::ClaudeProvider;
#[cfg(feature = "ollama")]
pub use ollama::OllamaProvider;
