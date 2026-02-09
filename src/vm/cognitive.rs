//! Cognitive Runtime - Abstraccion para ejecucion cognitiva del VM
//!
//! Define el trait que permite al VM llamar opcionalmente a un agente
//! durante la ejecucion para observar, deliberar y ajustar.

use super::{Value, RuntimeError, ExpectationFailure};
use std::fmt;

/// Decision del cognitive runtime despues de deliberar
#[derive(Debug, Clone)]
pub enum CognitiveDecision {
    /// Seguir sin cambios
    Continue,
    /// Inyectar valor en el flujo de ejecucion
    Override(Value),
    /// Generar patch al codigo
    Fix {
        new_code: String,
        explanation: String,
    },
    /// Volver a un checkpoint con ajustes
    Backtrack {
        checkpoint: String,
        adjustments: Vec<(String, Value)>,
    },
    /// Detener ejecucion
    Halt(RuntimeError),
}

/// Evento de observacion durante la ejecucion
#[derive(Debug, Clone)]
pub enum ObservationEvent {
    /// Una variable observada cambio de valor
    ValueChanged {
        name: String,
        old_value: Value,
        new_value: Value,
    },
    /// Un expect fue evaluado
    ExpectEvaluated {
        condition: String,
        result: bool,
        failure: Option<ExpectationFailure>,
    },
    /// Una funcion retorno un valor
    FunctionReturned {
        name: String,
        result: Value,
    },
    /// Se creo un checkpoint
    CheckpointCreated {
        name: String,
    },
}

/// Trigger que provoca deliberacion
#[derive(Debug, Clone)]
pub enum DeliberationTrigger {
    /// Un expect fallo
    ExpectFailed {
        failure: ExpectationFailure,
    },
    /// Bloque `reason` explicito
    ExplicitReason {
        observations: Vec<String>,
        question: String,
    },
    /// Error tecnico durante ejecucion
    TechnicalError {
        error: RuntimeError,
    },
    /// Un goal activo fallo su check
    GoalMisalignment {
        goal_description: String,
        check_result: Value,
    },
}

impl fmt::Display for DeliberationTrigger {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DeliberationTrigger::ExpectFailed { failure } => {
                write!(f, "expect failed: {}", failure)
            }
            DeliberationTrigger::ExplicitReason { question, .. } => {
                write!(f, "reason: {}", question)
            }
            DeliberationTrigger::TechnicalError { error } => {
                write!(f, "technical error: {}", error.message)
            }
            DeliberationTrigger::GoalMisalignment { goal_description, .. } => {
                write!(f, "goal misalignment: {}", goal_description)
            }
        }
    }
}

/// Trait para el runtime cognitivo
///
/// Cuando `is_active()` retorna false, todas las operaciones son no-op
/// y el compilador puede optimizar los checks a cero overhead.
pub trait CognitiveRuntime: Send {
    /// Notifica una observacion al runtime cognitivo
    fn observe(&mut self, event: ObservationEvent);

    /// Solicita deliberacion sobre una situacion
    fn deliberate(&mut self, trigger: DeliberationTrigger) -> CognitiveDecision;

    /// Evalua los goals activos contra el estado actual
    fn check_goals(&mut self) -> Vec<CognitiveDecision>;

    /// Indica si el runtime cognitivo esta activo
    fn is_active(&self) -> bool;

    /// Notifica los checkpoints disponibles para backtrack
    fn set_available_checkpoints(&mut self, _checkpoints: Vec<String>) {}
}

/// Implementacion nula del CognitiveRuntime
///
/// Todas las operaciones son no-op. `is_active()` retorna false.
/// Usado como default cuando no hay agente conectado (v1 behavior).
pub struct NullCognitiveRuntime;

impl CognitiveRuntime for NullCognitiveRuntime {
    fn observe(&mut self, _event: ObservationEvent) {
        // no-op
    }

    fn deliberate(&mut self, _trigger: DeliberationTrigger) -> CognitiveDecision {
        CognitiveDecision::Continue
    }

    fn check_goals(&mut self) -> Vec<CognitiveDecision> {
        Vec::new()
    }

    fn is_active(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_null_runtime_is_inactive() {
        let runtime = NullCognitiveRuntime;
        assert!(!runtime.is_active());
    }

    #[test]
    fn test_null_runtime_deliberate_returns_continue() {
        let mut runtime = NullCognitiveRuntime;
        let trigger = DeliberationTrigger::TechnicalError {
            error: RuntimeError::new("test"),
        };
        assert!(matches!(runtime.deliberate(trigger), CognitiveDecision::Continue));
    }

    #[test]
    fn test_null_runtime_check_goals_empty() {
        let mut runtime = NullCognitiveRuntime;
        assert!(runtime.check_goals().is_empty());
    }

    #[test]
    fn test_null_runtime_observe_noop() {
        let mut runtime = NullCognitiveRuntime;
        runtime.observe(ObservationEvent::CheckpointCreated {
            name: "test".to_string(),
        });
        // Should not panic
    }

    /// Mock cognitive runtime for testing
    struct MockCognitiveRuntime {
        observations: Vec<ObservationEvent>,
        decision: CognitiveDecision,
    }

    impl MockCognitiveRuntime {
        fn new(decision: CognitiveDecision) -> Self {
            Self {
                observations: Vec::new(),
                decision,
            }
        }
    }

    impl CognitiveRuntime for MockCognitiveRuntime {
        fn observe(&mut self, event: ObservationEvent) {
            self.observations.push(event);
        }

        fn deliberate(&mut self, _trigger: DeliberationTrigger) -> CognitiveDecision {
            self.decision.clone()
        }

        fn check_goals(&mut self) -> Vec<CognitiveDecision> {
            Vec::new()
        }

        fn is_active(&self) -> bool {
            true
        }
    }

    #[test]
    fn test_mock_runtime_is_active() {
        let runtime = MockCognitiveRuntime::new(CognitiveDecision::Continue);
        assert!(runtime.is_active());
    }

    #[test]
    fn test_mock_runtime_receives_observations() {
        let mut runtime = MockCognitiveRuntime::new(CognitiveDecision::Continue);
        runtime.observe(ObservationEvent::ValueChanged {
            name: "x".to_string(),
            old_value: Value::Nil,
            new_value: Value::Int(42),
        });
        assert_eq!(runtime.observations.len(), 1);
    }

    #[test]
    fn test_mock_runtime_returns_override() {
        let mut runtime = MockCognitiveRuntime::new(
            CognitiveDecision::Override(Value::Int(99))
        );
        let trigger = DeliberationTrigger::TechnicalError {
            error: RuntimeError::new("test"),
        };
        match runtime.deliberate(trigger) {
            CognitiveDecision::Override(Value::Int(99)) => {}
            other => panic!("Expected Override(99), got {:?}", other),
        }
    }
}
