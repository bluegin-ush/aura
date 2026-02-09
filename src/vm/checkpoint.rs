//! Checkpoint system for VM backtrack support
//!
//! Allows the VM to create snapshots of its state and restore to them,
//! enabling partial backtrack during cognitive execution.

use std::collections::HashMap;
use std::time::Instant;
use super::Value;

/// Snapshot of the VM state at a point in execution
#[derive(Debug, Clone)]
pub struct VMCheckpoint {
    /// Name of the checkpoint
    pub name: String,
    /// Copy of all variables at checkpoint time
    pub variables: HashMap<String, Value>,
    /// Step count at checkpoint time
    pub step_count: u64,
    /// When the checkpoint was created
    pub timestamp: Instant,
}

/// Manages checkpoints for the VM
pub struct CheckpointManager {
    checkpoints: HashMap<String, VMCheckpoint>,
    /// Ordered list of checkpoint names (insertion order)
    order: Vec<String>,
    /// Maximum number of checkpoints to keep
    max_checkpoints: usize,
}

impl CheckpointManager {
    /// Creates a new checkpoint manager with default max (10)
    pub fn new() -> Self {
        Self {
            checkpoints: HashMap::new(),
            order: Vec::new(),
            max_checkpoints: 10,
        }
    }

    /// Creates a new checkpoint manager with custom max
    pub fn with_max(max: usize) -> Self {
        Self {
            checkpoints: HashMap::new(),
            order: Vec::new(),
            max_checkpoints: max,
        }
    }

    /// Saves a checkpoint with the given name and variables
    pub fn save(&mut self, name: String, variables: HashMap<String, Value>, step_count: u64) {
        // If we're at max capacity and this is a new checkpoint, remove the oldest
        if !self.checkpoints.contains_key(&name) && self.checkpoints.len() >= self.max_checkpoints {
            if let Some(oldest) = self.order.first().cloned() {
                self.checkpoints.remove(&oldest);
                self.order.retain(|n| n != &oldest);
            }
        }

        // Remove from order if already exists (we'll re-add at end)
        self.order.retain(|n| n != &name);

        self.checkpoints.insert(name.clone(), VMCheckpoint {
            name: name.clone(),
            variables,
            step_count,
            timestamp: Instant::now(),
        });
        self.order.push(name);
    }

    /// Restores a checkpoint by name, returning the saved variables
    pub fn restore(&self, name: &str) -> Option<&VMCheckpoint> {
        self.checkpoints.get(name)
    }

    /// Checks if a checkpoint exists
    pub fn exists(&self, name: &str) -> bool {
        self.checkpoints.contains_key(name)
    }

    /// Returns names of all available checkpoints
    pub fn list(&self) -> Vec<String> {
        self.order.clone()
    }

    /// Returns the number of stored checkpoints
    pub fn count(&self) -> usize {
        self.checkpoints.len()
    }

    /// Returns the maximum number of checkpoints
    pub fn max_checkpoints(&self) -> usize {
        self.max_checkpoints
    }

    /// Clears all checkpoints
    pub fn clear(&mut self) {
        self.checkpoints.clear();
        self.order.clear();
    }

    /// Returns the most recent checkpoint name
    pub fn most_recent(&self) -> Option<&str> {
        self.order.last().map(|s| s.as_str())
    }
}

impl Default for CheckpointManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_save_and_restore_checkpoint() {
        let mut mgr = CheckpointManager::new();
        let mut vars = HashMap::new();
        vars.insert("x".to_string(), Value::Int(42));
        vars.insert("y".to_string(), Value::String("hello".to_string()));

        mgr.save("test_cp".to_string(), vars, 10);

        let cp = mgr.restore("test_cp").unwrap();
        assert_eq!(cp.name, "test_cp");
        assert_eq!(cp.variables.get("x"), Some(&Value::Int(42)));
        assert_eq!(cp.variables.get("y"), Some(&Value::String("hello".to_string())));
        assert_eq!(cp.step_count, 10);
    }

    #[test]
    fn test_restore_with_adjustments() {
        let mut mgr = CheckpointManager::new();
        let mut vars = HashMap::new();
        vars.insert("x".to_string(), Value::Int(42));
        vars.insert("y".to_string(), Value::Int(10));

        mgr.save("cp1".to_string(), vars, 5);

        // Restore and apply adjustments
        let cp = mgr.restore("cp1").unwrap();
        let mut restored_vars = cp.variables.clone();
        // Apply adjustment
        restored_vars.insert("x".to_string(), Value::Int(99));

        assert_eq!(restored_vars.get("x"), Some(&Value::Int(99)));
        assert_eq!(restored_vars.get("y"), Some(&Value::Int(10)));
    }

    #[test]
    fn test_max_checkpoints_respected() {
        let mut mgr = CheckpointManager::with_max(3);

        for i in 0..5 {
            let mut vars = HashMap::new();
            vars.insert("i".to_string(), Value::Int(i));
            mgr.save(format!("cp_{}", i), vars, i as u64);
        }

        // Should only keep 3 most recent
        assert_eq!(mgr.count(), 3);
        assert!(!mgr.exists("cp_0"));
        assert!(!mgr.exists("cp_1"));
        assert!(mgr.exists("cp_2"));
        assert!(mgr.exists("cp_3"));
        assert!(mgr.exists("cp_4"));
    }

    #[test]
    fn test_checkpoint_doesnt_affect_overwrite() {
        let mut mgr = CheckpointManager::new();

        let mut vars1 = HashMap::new();
        vars1.insert("x".to_string(), Value::Int(1));
        mgr.save("cp".to_string(), vars1, 1);

        let mut vars2 = HashMap::new();
        vars2.insert("x".to_string(), Value::Int(2));
        mgr.save("cp".to_string(), vars2, 2);

        // Overwritten checkpoint should have new value
        let cp = mgr.restore("cp").unwrap();
        assert_eq!(cp.variables.get("x"), Some(&Value::Int(2)));
        assert_eq!(mgr.count(), 1);
    }

    #[test]
    fn test_restore_nonexistent_returns_none() {
        let mgr = CheckpointManager::new();
        assert!(mgr.restore("nonexistent").is_none());
    }

    #[test]
    fn test_list_checkpoints() {
        let mut mgr = CheckpointManager::new();
        mgr.save("alpha".to_string(), HashMap::new(), 1);
        mgr.save("beta".to_string(), HashMap::new(), 2);

        let list = mgr.list();
        assert_eq!(list, vec!["alpha", "beta"]);
    }

    #[test]
    fn test_most_recent() {
        let mut mgr = CheckpointManager::new();
        assert!(mgr.most_recent().is_none());

        mgr.save("first".to_string(), HashMap::new(), 1);
        assert_eq!(mgr.most_recent(), Some("first"));

        mgr.save("second".to_string(), HashMap::new(), 2);
        assert_eq!(mgr.most_recent(), Some("second"));
    }
}
