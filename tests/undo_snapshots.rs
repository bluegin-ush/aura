//! Integration tests for undo and snapshots CLI commands.
//!
//! Tests that verify the undo and snapshots commands work correctly
//! with both JSON and human-readable output.

use std::process::Command;
use std::path::PathBuf;
use std::fs;
use std::sync::atomic::{AtomicU32, Ordering};

fn aura_binary() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("target");
    path.push("debug");
    path.push("aura");
    path
}

static TEST_COUNTER: AtomicU32 = AtomicU32::new(0);

fn test_dir_unique() -> PathBuf {
    let counter = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("target");
    path.push(format!("test_snapshots_{}", counter));
    path
}

/// Setup a clean test directory with unique name
fn setup_test_dir() -> PathBuf {
    let dir = test_dir_unique();
    if dir.exists() {
        fs::remove_dir_all(&dir).ok();
    }
    fs::create_dir_all(&dir).expect("Failed to create test directory");

    // Create .aura directory for snapshots
    let aura_dir = dir.join(".aura");
    fs::create_dir_all(aura_dir.join("snapshots")).expect("Failed to create snapshots directory");

    dir
}

/// Cleanup a specific test directory
fn cleanup_dir(dir: &PathBuf) {
    if dir.exists() {
        fs::remove_dir_all(dir).ok();
    }
}

mod undo_command {
    use super::*;

    #[test]
    fn test_undo_list_empty_json() {
        let dir = setup_test_dir();

        let output = Command::new(aura_binary())
            .current_dir(&dir)
            .args(["undo", "--list", "--json"])
            .output()
            .expect("Failed to execute aura undo --list");

        let stdout = String::from_utf8_lossy(&output.stdout);
        let json: serde_json::Value = serde_json::from_str(&stdout)
            .expect("Output should be valid JSON");

        assert_eq!(json["success"], true);
        assert!(json["actions"].as_array().unwrap().is_empty());

        cleanup_dir(&dir);
    }

    #[test]
    fn test_undo_list_empty_text() {
        let dir = setup_test_dir();

        let output = Command::new(aura_binary())
            .current_dir(&dir)
            .args(["undo", "--list"])
            .output()
            .expect("Failed to execute aura undo --list");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("No actions") || stdout.contains("history"));

        cleanup_dir(&dir);
    }

    #[test]
    fn test_undo_nothing_to_undo_json() {
        let dir = setup_test_dir();

        let output = Command::new(aura_binary())
            .current_dir(&dir)
            .args(["undo", "--json"])
            .output()
            .expect("Failed to execute aura undo");

        let stdout = String::from_utf8_lossy(&output.stdout);
        let json: serde_json::Value = serde_json::from_str(&stdout)
            .expect("Output should be valid JSON");

        assert_eq!(json["success"], false);
        assert!(json["error"].as_str().is_some());

        cleanup_dir(&dir);
    }

    #[test]
    fn test_undo_to_nonexistent_json() {
        let dir = setup_test_dir();

        let output = Command::new(aura_binary())
            .current_dir(&dir)
            .args(["undo", "--to", "nonexistent_snap", "--json"])
            .output()
            .expect("Failed to execute aura undo --to");

        let stdout = String::from_utf8_lossy(&output.stdout);
        let json: serde_json::Value = serde_json::from_str(&stdout)
            .expect("Output should be valid JSON");

        assert_eq!(json["success"], false);

        cleanup_dir(&dir);
    }
}

mod snapshots_command {
    use super::*;

    #[test]
    fn test_snapshots_list_empty_json() {
        let dir = setup_test_dir();

        let output = Command::new(aura_binary())
            .current_dir(&dir)
            .args(["snapshots", "--json"])
            .output()
            .expect("Failed to execute aura snapshots");

        let stdout = String::from_utf8_lossy(&output.stdout);
        let json: serde_json::Value = serde_json::from_str(&stdout)
            .expect("Output should be valid JSON");

        assert_eq!(json["success"], true);
        assert!(json["snapshots"].as_array().unwrap().is_empty());

        cleanup_dir(&dir);
    }

    #[test]
    fn test_snapshots_list_empty_text() {
        let dir = setup_test_dir();

        let output = Command::new(aura_binary())
            .current_dir(&dir)
            .args(["snapshots"])
            .output()
            .expect("Failed to execute aura snapshots");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("No snapshots"));

        cleanup_dir(&dir);
    }

    #[test]
    fn test_snapshots_create_json() {
        let dir = setup_test_dir();

        // Create a test file
        let test_file = dir.join("test.aura");
        fs::write(&test_file, "x = 42").expect("Failed to write test file");

        let output = Command::new(aura_binary())
            .current_dir(&dir)
            .args(["snapshots", "create", "--json"])
            .arg(&test_file)
            .output()
            .expect("Failed to execute aura snapshots create");

        let stdout = String::from_utf8_lossy(&output.stdout);
        let json: serde_json::Value = serde_json::from_str(&stdout)
            .expect("Output should be valid JSON");

        assert_eq!(json["success"], true);
        assert!(json["id"].as_str().is_some());
        assert!(json["timestamp"].as_u64().is_some());
        assert!(!json["files"].as_array().unwrap().is_empty());

        cleanup_dir(&dir);
    }

    #[test]
    fn test_snapshots_create_with_description_json() {
        let dir = setup_test_dir();

        // Create a test file
        let test_file = dir.join("test.aura");
        fs::write(&test_file, "x = 42").expect("Failed to write test file");

        let output = Command::new(aura_binary())
            .current_dir(&dir)
            .args(["snapshots", "create", "-d", "Test snapshot", "--json"])
            .arg(&test_file)
            .output()
            .expect("Failed to execute aura snapshots create");

        let stdout = String::from_utf8_lossy(&output.stdout);
        let json: serde_json::Value = serde_json::from_str(&stdout)
            .expect("Output should be valid JSON");

        assert_eq!(json["success"], true);

        cleanup_dir(&dir);
    }

    #[test]
    fn test_snapshots_create_no_files_json() {
        let dir = setup_test_dir();

        // No .aura files in directory
        let output = Command::new(aura_binary())
            .current_dir(&dir)
            .args(["snapshots", "create", "--json"])
            .output()
            .expect("Failed to execute aura snapshots create");

        let stdout = String::from_utf8_lossy(&output.stdout);
        let json: serde_json::Value = serde_json::from_str(&stdout)
            .expect("Output should be valid JSON");

        assert_eq!(json["success"], false);
        assert!(json["error"].as_str().is_some());

        cleanup_dir(&dir);
    }

    #[test]
    fn test_snapshots_restore_nonexistent_json() {
        let dir = setup_test_dir();

        let output = Command::new(aura_binary())
            .current_dir(&dir)
            .args(["snapshots", "restore", "nonexistent_snap", "--json"])
            .output()
            .expect("Failed to execute aura snapshots restore");

        let stdout = String::from_utf8_lossy(&output.stdout);
        let json: serde_json::Value = serde_json::from_str(&stdout)
            .expect("Output should be valid JSON");

        assert_eq!(json["success"], false);
        assert!(json["error"].as_str().is_some());

        cleanup_dir(&dir);
    }

    #[test]
    fn test_snapshots_create_and_restore_json() {
        let dir = setup_test_dir();

        // Create a test file
        let test_file = dir.join("test.aura");
        fs::write(&test_file, "x = 42").expect("Failed to write test file");

        // Create snapshot
        let create_output = Command::new(aura_binary())
            .current_dir(&dir)
            .args(["snapshots", "create", "--json"])
            .arg(&test_file)
            .output()
            .expect("Failed to execute aura snapshots create");

        let create_stdout = String::from_utf8_lossy(&create_output.stdout);
        let create_json: serde_json::Value = serde_json::from_str(&create_stdout)
            .expect("Create output should be valid JSON");

        assert_eq!(create_json["success"], true);
        let snapshot_id = create_json["id"].as_str().unwrap();

        // Modify the file
        fs::write(&test_file, "x = 100").expect("Failed to modify test file");

        // Verify file was modified
        let content = fs::read_to_string(&test_file).unwrap();
        assert_eq!(content, "x = 100");

        // Restore snapshot
        let restore_output = Command::new(aura_binary())
            .current_dir(&dir)
            .args(["snapshots", "restore", snapshot_id, "--json"])
            .output()
            .expect("Failed to execute aura snapshots restore");

        let restore_stdout = String::from_utf8_lossy(&restore_output.stdout);
        let restore_json: serde_json::Value = serde_json::from_str(&restore_stdout)
            .expect("Restore output should be valid JSON");

        assert_eq!(restore_json["success"], true);
        assert!(!restore_json["files_restored"].as_array().unwrap().is_empty());

        // Verify file was restored
        let restored_content = fs::read_to_string(&test_file).unwrap();
        assert_eq!(restored_content, "x = 42");

        cleanup_dir(&dir);
    }

    #[test]
    fn test_snapshots_prune_json() {
        let dir = setup_test_dir();

        // Create a test file
        let test_file = dir.join("test.aura");
        fs::write(&test_file, "x = 42").expect("Failed to write test file");

        // Create multiple snapshots
        for i in 0..3 {
            fs::write(&test_file, format!("x = {}", i)).expect("Failed to write test file");

            Command::new(aura_binary())
                .current_dir(&dir)
                .args(["snapshots", "create", "--json"])
                .arg(&test_file)
                .output()
                .expect("Failed to execute aura snapshots create");

            // Small delay to ensure unique timestamps
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        // Prune to keep only 1
        let output = Command::new(aura_binary())
            .current_dir(&dir)
            .args(["snapshots", "prune", "--keep", "1", "--json"])
            .output()
            .expect("Failed to execute aura snapshots prune");

        let stdout = String::from_utf8_lossy(&output.stdout);
        let json: serde_json::Value = serde_json::from_str(&stdout)
            .expect("Output should be valid JSON");

        assert_eq!(json["success"], true);
        assert_eq!(json["removed_count"], 2);
        assert_eq!(json["remaining_count"], 1);

        cleanup_dir(&dir);
    }

    #[test]
    fn test_snapshots_list_after_create_json() {
        let dir = setup_test_dir();

        // Create a test file
        let test_file = dir.join("test.aura");
        fs::write(&test_file, "x = 42").expect("Failed to write test file");

        // Create snapshot
        Command::new(aura_binary())
            .current_dir(&dir)
            .args(["snapshots", "create", "-d", "Test", "--json"])
            .arg(&test_file)
            .output()
            .expect("Failed to execute aura snapshots create");

        // List snapshots
        let output = Command::new(aura_binary())
            .current_dir(&dir)
            .args(["snapshots", "--json"])
            .output()
            .expect("Failed to execute aura snapshots");

        let stdout = String::from_utf8_lossy(&output.stdout);
        let json: serde_json::Value = serde_json::from_str(&stdout)
            .expect("Output should be valid JSON");

        assert_eq!(json["success"], true);
        let snapshots = json["snapshots"].as_array().unwrap();
        assert_eq!(snapshots.len(), 1);

        let snapshot = &snapshots[0];
        assert!(snapshot["id"].as_str().is_some());
        assert!(snapshot["timestamp"].as_u64().is_some());
        assert!(snapshot["reason"].as_str().is_some());
        assert!(!snapshot["files"].as_array().unwrap().is_empty());

        cleanup_dir(&dir);
    }
}
