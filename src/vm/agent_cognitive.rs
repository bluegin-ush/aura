//! AgentCognitiveRuntime - Real implementation using AgentProvider
//!
//! Connects the CognitiveRuntime trait to an actual AI agent
//! through the AgentProvider interface.

use tokio::runtime::Handle;

use super::cognitive::*;
use super::{Value, RuntimeError};
use crate::agent::{AgentProvider, AgentRequest, AgentResponse, Action, EventType};
use crate::parser::GoalDef;

/// Episode of reasoning for memory/tracing
#[derive(Debug, Clone)]
pub struct ReasoningEpisode {
    pub trigger_type: String,
    pub observations: Vec<String>,
    pub decision: String,
    pub decision_detail: String,
    pub outcome: Option<EpisodeOutcome>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub context: EpisodeContext,
}

/// Outcome of a reasoning episode
#[derive(Debug, Clone)]
pub enum EpisodeOutcome {
    Success,
    Failure(String),
    Pending,
}

/// Context for a reasoning episode
#[derive(Debug, Clone)]
pub struct EpisodeContext {
    pub file: String,
    pub function: Option<String>,
    pub goals: Vec<String>,
}

/// Safety configuration for cognitive runtime
#[derive(Debug, Clone)]
pub struct CognitiveSafetyConfig {
    /// Maximum number of lines a fix can change
    pub max_fix_lines: usize,
    /// Maximum consecutive backtracks before stopping
    pub max_backtrack_depth: usize,
    /// Maximum deliberations without progress before stopping
    pub max_deliberations_without_progress: usize,
}

impl Default for CognitiveSafetyConfig {
    fn default() -> Self {
        Self {
            max_fix_lines: 50,
            max_backtrack_depth: 5,
            max_deliberations_without_progress: 3,
        }
    }
}

/// Validates a proposed fix against safety rules
pub fn validate_fix(
    new_code: &str,
    original_goals: &[GoalDef],
    config: &CognitiveSafetyConfig,
) -> Result<(), String> {
    // Check fix size
    let line_count = new_code.lines().count();
    if line_count > config.max_fix_lines {
        return Err(format!(
            "Fix too large: {} lines (max {})",
            line_count, config.max_fix_lines
        ));
    }

    // Check that goals are not modified
    let tokens = crate::lexer::tokenize(new_code).map_err(|e| format!("Fix doesn't tokenize: {:?}", e))?;
    let program = crate::parser::parse(tokens).map_err(|e| {
        let msgs: Vec<String> = e.iter().map(|err| err.message.clone()).collect();
        format!("Fix doesn't parse: {}", msgs.join(", "))
    })?;

    let new_goals: Vec<&GoalDef> = program.definitions.iter().filter_map(|d| {
        if let crate::parser::Definition::Goal(g) = d { Some(g) } else { None }
    }).collect();

    // Verify goals haven't been tampered with
    let original_descs: Vec<&str> = original_goals.iter().map(|g| g.description.as_str()).collect();
    for new_goal in &new_goals {
        if !original_descs.contains(&new_goal.description.as_str()) {
            return Err(format!(
                "Fix introduces new goal: '{}'. Goals cannot be modified by fixes.",
                new_goal.description
            ));
        }
    }

    // Check that original goals still exist
    let new_descs: Vec<&str> = new_goals.iter().map(|g| g.description.as_str()).collect();
    for orig in &original_descs {
        if !new_descs.contains(orig) {
            return Err(format!(
                "Fix removes goal: '{}'. Goals cannot be removed by fixes.",
                orig
            ));
        }
    }

    Ok(())
}

/// Cognitive runtime backed by an AgentProvider
pub struct AgentCognitiveRuntime<P: AgentProvider> {
    provider: P,
    tokio_handle: Handle,
    goals: Vec<GoalDef>,
    invariants: Vec<String>,
    source_code: String,
    observation_buffer: Vec<ObservationEvent>,
    pub reasoning_trace: Vec<ReasoningEpisode>,
    available_checkpoints: Vec<String>,
    max_deliberations: usize,
    deliberation_count: usize,
    safety_config: CognitiveSafetyConfig,
    consecutive_backtracks: usize,
    deliberations_without_progress: usize,
}

impl<P: AgentProvider> AgentCognitiveRuntime<P> {
    /// Creates a new AgentCognitiveRuntime
    pub fn new(
        provider: P,
        tokio_handle: Handle,
        goals: Vec<GoalDef>,
        invariants: Vec<String>,
        source_code: String,
    ) -> Self {
        Self {
            provider,
            tokio_handle,
            goals,
            invariants,
            source_code,
            observation_buffer: Vec::new(),
            reasoning_trace: Vec::new(),
            available_checkpoints: Vec::new(),
            max_deliberations: 10,
            deliberation_count: 0,
            safety_config: CognitiveSafetyConfig::default(),
            consecutive_backtracks: 0,
            deliberations_without_progress: 0,
        }
    }

    /// Sets the safety configuration
    pub fn with_safety_config(mut self, config: CognitiveSafetyConfig) -> Self {
        self.safety_config = config;
        self
    }

    /// Sets the maximum number of deliberations
    pub fn with_max_deliberations(mut self, max: usize) -> Self {
        self.max_deliberations = max;
        self
    }

    /// Builds a deliberation request for the agent
    fn build_deliberation_request(&self, trigger: &DeliberationTrigger) -> AgentRequest {
        let mut message_parts = vec![format!("DELIBERATION TRIGGER: {}", trigger)];

        // Add observations
        if !self.observation_buffer.is_empty() {
            message_parts.push("\nRecent observations:".to_string());
            for obs in &self.observation_buffer {
                message_parts.push(format!("  - {:?}", obs));
            }
        }

        // Add goals
        if !self.goals.is_empty() {
            message_parts.push("\nProgram goals:".to_string());
            for goal in &self.goals {
                let check_str = if goal.check.is_some() { " [ACTIVE]" } else { "" };
                message_parts.push(format!("  - {}{}", goal.description, check_str));
            }
        }

        // Add invariants
        if !self.invariants.is_empty() {
            message_parts.push("\nInvariants (MUST respect):".to_string());
            for inv in &self.invariants {
                message_parts.push(format!("  - {}", inv));
            }
        }

        // Add available checkpoints for backtrack
        if !self.available_checkpoints.is_empty() {
            message_parts.push("\nAvailable checkpoints for backtrack:".to_string());
            for cp in &self.available_checkpoints {
                message_parts.push(format!("  - {}", cp));
            }
        }

        // Add reasoning trace for context
        if !self.reasoning_trace.is_empty() {
            let recent: Vec<_> = self.reasoning_trace.iter().rev().take(3).collect();
            message_parts.push("\nRecent reasoning episodes:".to_string());
            for ep in recent {
                message_parts.push(format!("  - {} -> {} ({})", ep.trigger_type, ep.decision, ep.decision_detail));
            }
        }

        let message = message_parts.join("\n");

        AgentRequest::new(EventType::Error)
            .with_context(&self.source_code)
            .with_message(&message)
    }

    /// Maps an AgentResponse to a CognitiveDecision
    fn map_response(&self, response: AgentResponse) -> CognitiveDecision {
        match response.action {
            Action::Patch => {
                if let Some(patch) = response.patch {
                    CognitiveDecision::Fix {
                        new_code: patch.new_code,
                        explanation: response.explanation,
                    }
                } else {
                    CognitiveDecision::Continue
                }
            }
            Action::Generate => {
                if let Some(code) = response.generated_code {
                    // Try to parse as a Value for Override
                    CognitiveDecision::Override(Value::String(code))
                } else {
                    CognitiveDecision::Continue
                }
            }
            Action::Suggest => CognitiveDecision::Continue,
            Action::Clarify => {
                CognitiveDecision::Halt(RuntimeError::new(
                    format!("Agent needs clarification: {}", response.explanation)
                ))
            }
            Action::Escalate => {
                CognitiveDecision::Halt(RuntimeError::new(
                    format!("Agent escalated: {}", response.escalation_reason.unwrap_or(response.explanation))
                ))
            }
        }
    }

    /// Records a reasoning episode
    fn record_episode(&mut self, trigger: &DeliberationTrigger, decision: &CognitiveDecision) {
        let trigger_type = match trigger {
            DeliberationTrigger::ExpectFailed { .. } => "expect_failed",
            DeliberationTrigger::ExplicitReason { .. } => "reason",
            DeliberationTrigger::TechnicalError { .. } => "technical_error",
            DeliberationTrigger::GoalMisalignment { .. } => "goal_misalignment",
        };

        let (decision_str, detail) = match decision {
            CognitiveDecision::Continue => ("continue", String::new()),
            CognitiveDecision::Override(val) => ("override", format!("{}", val)),
            CognitiveDecision::Fix { explanation, .. } => ("fix", explanation.clone()),
            CognitiveDecision::Backtrack { checkpoint, .. } => ("backtrack", checkpoint.clone()),
            CognitiveDecision::Halt(err) => ("halt", err.message.clone()),
        };

        let observations: Vec<String> = self.observation_buffer.iter()
            .map(|o| format!("{:?}", o))
            .collect();

        self.reasoning_trace.push(ReasoningEpisode {
            trigger_type: trigger_type.to_string(),
            observations,
            decision: decision_str.to_string(),
            decision_detail: detail,
            outcome: None,
            timestamp: chrono::Utc::now(),
            context: EpisodeContext {
                file: String::new(),
                function: None,
                goals: self.goals.iter().map(|g| g.description.clone()).collect(),
            },
        });
    }
}

impl<P: AgentProvider> CognitiveRuntime for AgentCognitiveRuntime<P> {
    fn observe(&mut self, event: ObservationEvent) {
        self.observation_buffer.push(event);
    }

    fn deliberate(&mut self, trigger: DeliberationTrigger) -> CognitiveDecision {
        // Check safety limits
        if self.deliberation_count >= self.max_deliberations {
            return CognitiveDecision::Continue;
        }
        self.deliberation_count += 1;

        // Build and send request
        let request = self.build_deliberation_request(&trigger);

        let mut decision = match self.tokio_handle.block_on(self.provider.send_request(request)) {
            Ok(response) => self.map_response(response),
            Err(_) => {
                // Fail-open: if provider fails, continue
                CognitiveDecision::Continue
            }
        };

        // Safety: validate fixes
        if let CognitiveDecision::Fix { ref new_code, .. } = decision {
            if let Err(reason) = validate_fix(new_code, &self.goals, &self.safety_config) {
                decision = CognitiveDecision::Continue;
                self.deliberations_without_progress += 1;
                // Log rejection (visible in trace)
                self.reasoning_trace.push(ReasoningEpisode {
                    trigger_type: "safety_rejected".to_string(),
                    observations: vec![reason],
                    decision: "continue".to_string(),
                    decision_detail: "fix rejected by safety validation".to_string(),
                    outcome: None,
                    timestamp: chrono::Utc::now(),
                    context: EpisodeContext {
                        file: String::new(),
                        function: None,
                        goals: self.goals.iter().map(|g| g.description.clone()).collect(),
                    },
                });
            } else {
                self.consecutive_backtracks = 0;
                self.deliberations_without_progress = 0;
            }
        }

        // Safety: check backtrack depth
        if let CognitiveDecision::Backtrack { ref checkpoint, .. } = decision {
            self.consecutive_backtracks += 1;
            if self.consecutive_backtracks > self.safety_config.max_backtrack_depth {
                decision = CognitiveDecision::Continue;
                self.deliberations_without_progress += 1;
            } else if !self.available_checkpoints.contains(checkpoint) {
                decision = CognitiveDecision::Continue;
                self.deliberations_without_progress += 1;
            } else {
                self.deliberations_without_progress = 0;
            }
        }

        // Safety: track progress
        if matches!(decision, CognitiveDecision::Continue) {
            // Continue is fine but counts as no progress if we expected action
        }

        // Safety: stop if too many deliberations without progress
        if self.deliberations_without_progress >= self.safety_config.max_deliberations_without_progress {
            decision = CognitiveDecision::Continue;
        }

        // Record episode
        self.record_episode(&trigger, &decision);

        // Drain observation buffer after deliberation
        self.observation_buffer.clear();

        decision
    }

    fn check_goals(&mut self) -> Vec<CognitiveDecision> {
        // Goals are evaluated by the VM, not the runtime
        Vec::new()
    }

    fn is_active(&self) -> bool {
        true
    }

    fn set_available_checkpoints(&mut self, checkpoints: Vec<String>) {
        self.available_checkpoints = checkpoints;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::MockProvider;
    use crate::parser::GoalDef;
    use crate::lexer::Span;

    fn make_handle() -> Handle {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.handle().clone()
    }

    #[test]
    fn test_agent_cognitive_is_active() {
        let provider = MockProvider::new().with_latency(0);
        let handle = make_handle();
        let runtime = AgentCognitiveRuntime::new(
            provider, handle, Vec::new(), Vec::new(), String::new(),
        );
        assert!(runtime.is_active());
    }

    #[test]
    fn test_agent_cognitive_max_deliberations() {
        let provider = MockProvider::new().with_latency(0);
        let handle = make_handle();
        let mut runtime = AgentCognitiveRuntime::new(
            provider, handle, Vec::new(), Vec::new(), String::new(),
        ).with_max_deliberations(2);

        let trigger = DeliberationTrigger::TechnicalError {
            error: RuntimeError::new("test"),
        };

        // First two should work
        runtime.deliberate(trigger.clone());
        runtime.deliberate(trigger.clone());

        // Third should return Continue (safety limit)
        let decision = runtime.deliberate(trigger);
        assert!(matches!(decision, CognitiveDecision::Continue));
    }

    #[test]
    fn test_agent_cognitive_records_episodes() {
        let provider = MockProvider::new().with_latency(0);
        let handle = make_handle();
        let mut runtime = AgentCognitiveRuntime::new(
            provider, handle, Vec::new(), Vec::new(), String::new(),
        );

        let trigger = DeliberationTrigger::TechnicalError {
            error: RuntimeError::new("test error"),
        };
        runtime.deliberate(trigger);

        assert_eq!(runtime.reasoning_trace.len(), 1);
        assert_eq!(runtime.reasoning_trace[0].trigger_type, "technical_error");
    }

    #[test]
    fn test_agent_cognitive_observation_buffer() {
        let provider = MockProvider::new().with_latency(0);
        let handle = make_handle();
        let mut runtime = AgentCognitiveRuntime::new(
            provider, handle, Vec::new(), Vec::new(), String::new(),
        );

        runtime.observe(ObservationEvent::ValueChanged {
            name: "x".to_string(),
            old_value: Value::Nil,
            new_value: Value::Int(42),
        });

        assert_eq!(runtime.observation_buffer.len(), 1);

        // After deliberation, buffer should be drained
        let trigger = DeliberationTrigger::TechnicalError {
            error: RuntimeError::new("test"),
        };
        runtime.deliberate(trigger);

        assert!(runtime.observation_buffer.is_empty());
    }

    #[test]
    fn test_agent_cognitive_with_checkpoints() {
        let provider = MockProvider::new().with_latency(0);
        let handle = make_handle();
        let mut runtime = AgentCognitiveRuntime::new(
            provider, handle, Vec::new(), Vec::new(), String::new(),
        );

        runtime.set_available_checkpoints(vec![
            "cp1".to_string(),
            "cp2".to_string(),
        ]);

        assert_eq!(runtime.available_checkpoints.len(), 2);
    }

    #[test]
    fn test_validate_fix_valid() {
        let goals = vec![];
        let config = CognitiveSafetyConfig::default();
        let code = "+http\nmain = 42\n";
        assert!(validate_fix(code, &goals, &config).is_ok());
    }

    #[test]
    fn test_validate_fix_too_large() {
        let goals = vec![];
        let config = CognitiveSafetyConfig { max_fix_lines: 3, ..Default::default() };
        let code = "+http\na = 1\nb = 2\nc = 3\nmain = a + b + c\n";
        assert!(validate_fix(code, &goals, &config).is_err());
    }

    #[test]
    fn test_validate_fix_preserves_goals() {
        use crate::lexer::Span;
        let goals = vec![
            GoalDef { description: "test goal".to_string(), check: None, span: Span::new(0, 0) },
        ];
        let config = CognitiveSafetyConfig::default();
        let code = "+http\ngoal \"test goal\"\nmain = 42\n";
        assert!(validate_fix(code, &goals, &config).is_ok());
    }

    #[test]
    fn test_validate_fix_rejects_removed_goal() {
        use crate::lexer::Span;
        let goals = vec![
            GoalDef { description: "important goal".to_string(), check: None, span: Span::new(0, 0) },
        ];
        let config = CognitiveSafetyConfig::default();
        // Fix code has no goals
        let code = "+http\nmain = 42\n";
        let result = validate_fix(code, &goals, &config);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("removes goal"));
    }

    #[test]
    fn test_validate_fix_rejects_new_goal() {
        let goals = vec![];
        let config = CognitiveSafetyConfig::default();
        let code = "+http\ngoal \"sneaky goal\"\nmain = 42\n";
        let result = validate_fix(code, &goals, &config);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("introduces new goal"));
    }

    #[test]
    fn test_validate_fix_rejects_unparseable() {
        let goals = vec![];
        let config = CognitiveSafetyConfig::default();
        let code = "this is not valid aura code @@@@";
        assert!(validate_fix(code, &goals, &config).is_err());
    }

    #[test]
    fn test_safety_config_defaults() {
        let config = CognitiveSafetyConfig::default();
        assert_eq!(config.max_fix_lines, 50);
        assert_eq!(config.max_backtrack_depth, 5);
        assert_eq!(config.max_deliberations_without_progress, 3);
    }
}
