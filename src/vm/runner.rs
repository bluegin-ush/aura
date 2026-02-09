//! Cognitive execution runner
//!
//! Provides `run_cognitive()` which wraps the VM execution with
//! retry logic and fix application for cognitive mode.

use crate::parser::Program;
use super::{VM, Value, RuntimeError};
use super::cognitive::CognitiveRuntime;

/// Result of a cognitive run
#[derive(Debug)]
pub struct CognitiveRunResult {
    /// The final value returned by the program
    pub value: Value,
    /// Any fixes that were applied during execution
    pub applied_fixes: Vec<(String, String)>,
    /// Number of retries that were needed
    pub retries: usize,
}

/// Runs a program with cognitive runtime support
///
/// For each attempt:
/// 1. Loads and runs the program
/// 2. If there are pending_fixes, applies them and re-parses
/// 3. If the result is Ok and no fixes pending, returns
/// 4. If max_retries exhausted, returns the error
pub fn run_cognitive(
    source: &str,
    cognitive: Box<dyn CognitiveRuntime>,
    max_retries: usize,
) -> Result<CognitiveRunResult, RuntimeError> {
    let mut current_source = source.to_string();
    let mut all_fixes = Vec::new();
    let mut cognitive_opt: Option<Box<dyn CognitiveRuntime>> = Some(cognitive);

    for attempt in 0..=max_retries {
        // Parse the current source
        let tokens = crate::lexer::tokenize(&current_source)
            .map_err(|e| RuntimeError::new(format!("Tokenize error: {:?}", e)))?;
        let program = crate::parser::parse(tokens)
            .map_err(|errors| {
                let msgs: Vec<String> = errors.iter().map(|e| e.message.clone()).collect();
                RuntimeError::new(format!("Parse errors: {}", msgs.join(", ")))
            })?;

        // Create VM - use cognitive runtime on first attempt, NullCognitiveRuntime for retries
        let mut vm = if let Some(cog) = cognitive_opt.take() {
            VM::with_cognitive(cog)
        } else {
            VM::new()
        };

        vm.load(&program);
        let result = vm.run();

        // Check for pending fixes
        if !vm.pending_fixes.is_empty() {
            let fixes = vm.pending_fixes.clone();
            let goals = vm.get_goals().to_vec();
            let safety = super::agent_cognitive::CognitiveSafetyConfig::default();
            for (new_code, explanation) in &fixes {
                // Validate fix before applying
                if let Err(reason) = super::agent_cognitive::validate_fix(new_code, &goals, &safety) {
                    eprintln!("Fix rejected: {}", reason);
                    continue;
                }
                current_source = new_code.clone();
                all_fixes.push((new_code.clone(), explanation.clone()));
            }
            if attempt < max_retries {
                continue;
            }
        }

        match result {
            Ok(value) => {
                return Ok(CognitiveRunResult {
                    value,
                    applied_fixes: all_fixes,
                    retries: attempt,
                });
            }
            Err(err) => {
                if attempt >= max_retries {
                    return Err(err);
                }
            }
        }
    }

    Err(RuntimeError::new("Cognitive run exhausted all retries"))
}

/// Simplified version that takes a pre-parsed program
pub fn run_program_cognitive(
    program: &Program,
    cognitive: Box<dyn CognitiveRuntime>,
) -> Result<CognitiveRunResult, RuntimeError> {
    let mut vm = VM::with_cognitive(cognitive);
    vm.load(program);
    let result = vm.run()?;

    Ok(CognitiveRunResult {
        value: result,
        applied_fixes: vm.pending_fixes.clone(),
        retries: 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vm::cognitive::NullCognitiveRuntime;

    #[test]
    fn test_run_cognitive_simple() {
        let source = "+http\nmain = 42\n";
        let result = run_cognitive(source, Box::new(NullCognitiveRuntime), 3);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().value, Value::Int(42));
    }

    #[test]
    fn test_run_program_cognitive_simple() {
        let tokens = crate::lexer::tokenize("+http\nmain = 42\n").unwrap();
        let program = crate::parser::parse(tokens).unwrap();
        let result = run_program_cognitive(&program, Box::new(NullCognitiveRuntime));
        assert!(result.is_ok());
        assert_eq!(result.unwrap().value, Value::Int(42));
    }
}
