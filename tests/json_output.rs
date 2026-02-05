//! Integration tests for JSON output format.
//!
//! Tests that verify the --json flag produces correct structured output
//! for all AURA CLI commands.

use std::process::Command;
use std::path::PathBuf;

fn aura_binary() -> PathBuf {
    // Get the path to the built binary
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("target");
    path.push("debug");
    path.push("aura");
    path
}

fn examples_dir() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("examples");
    path
}

mod check_command {
    use super::*;

    #[test]
    fn test_check_success_json() {
        let output = Command::new(aura_binary())
            .args(["check", "--json"])
            .arg(examples_dir().join("simple.aura"))
            .output()
            .expect("Failed to execute aura check");

        let stdout = String::from_utf8_lossy(&output.stdout);
        let json: serde_json::Value = serde_json::from_str(&stdout)
            .expect("Output should be valid JSON");

        assert_eq!(json["success"], true);
        assert!(json["file"].as_str().unwrap().contains("simple.aura"));
        assert!(json["errors"].as_array().unwrap().is_empty());
        assert!(json["stats"]["capabilities"].as_u64().is_some());
        assert!(json["stats"]["definitions"].as_u64().is_some());
    }

    #[test]
    fn test_check_error_json() {
        let output = Command::new(aura_binary())
            .args(["check", "--json"])
            .arg(examples_dir().join("errors.aura"))
            .output()
            .expect("Failed to execute aura check");

        let stdout = String::from_utf8_lossy(&output.stdout);
        let json: serde_json::Value = serde_json::from_str(&stdout)
            .expect("Output should be valid JSON");

        assert_eq!(json["success"], false);
        assert!(json["file"].as_str().unwrap().contains("errors.aura"));

        let errors = json["errors"].as_array().unwrap();
        assert!(!errors.is_empty());

        // Check error structure
        let first_error = &errors[0];
        assert!(first_error["code"].as_str().is_some());
        assert!(first_error["message"].as_str().is_some());
    }

    #[test]
    fn test_check_nonexistent_file_json() {
        let output = Command::new(aura_binary())
            .args(["check", "--json", "nonexistent.aura"])
            .output()
            .expect("Failed to execute aura check");

        let stdout = String::from_utf8_lossy(&output.stdout);
        let json: serde_json::Value = serde_json::from_str(&stdout)
            .expect("Output should be valid JSON");

        assert_eq!(json["success"], false);
        assert!(!json["errors"].as_array().unwrap().is_empty());
    }
}

mod run_command {
    use super::*;

    #[test]
    fn test_run_success_json() {
        let output = Command::new(aura_binary())
            .args(["run", "--json"])
            .arg(examples_dir().join("simple.aura"))
            .output()
            .expect("Failed to execute aura run");

        let stdout = String::from_utf8_lossy(&output.stdout);
        let json: serde_json::Value = serde_json::from_str(&stdout)
            .expect("Output should be valid JSON");

        assert_eq!(json["success"], true);
        assert!(json["result"].is_string());
        assert_eq!(json["type"], "String");
        assert!(json["duration_ms"].as_u64().is_some());
    }

    #[test]
    fn test_run_error_json() {
        let output = Command::new(aura_binary())
            .args(["run", "--json"])
            .arg(examples_dir().join("errors.aura"))
            .output()
            .expect("Failed to execute aura run");

        let stdout = String::from_utf8_lossy(&output.stdout);
        let json: serde_json::Value = serde_json::from_str(&stdout)
            .expect("Output should be valid JSON");

        assert_eq!(json["success"], false);
        assert!(json["error"].is_object());
        assert!(json["error"]["code"].as_str().is_some());
        assert!(json["error"]["message"].as_str().is_some());
    }

    #[test]
    fn test_run_nonexistent_file_json() {
        let output = Command::new(aura_binary())
            .args(["run", "--json", "nonexistent.aura"])
            .output()
            .expect("Failed to execute aura run");

        let stdout = String::from_utf8_lossy(&output.stdout);
        let json: serde_json::Value = serde_json::from_str(&stdout)
            .expect("Output should be valid JSON");

        assert_eq!(json["success"], false);
        assert!(json["error"].is_object());
    }
}

mod lex_command {
    use super::*;

    #[test]
    fn test_lex_json() {
        let output = Command::new(aura_binary())
            .args(["lex", "--json"])
            .arg(examples_dir().join("simple.aura"))
            .output()
            .expect("Failed to execute aura lex");

        let stdout = String::from_utf8_lossy(&output.stdout);
        let json: serde_json::Value = serde_json::from_str(&stdout)
            .expect("Output should be valid JSON");

        // Should be an array of tokens
        assert!(json.is_array());
        let tokens = json.as_array().unwrap();
        assert!(!tokens.is_empty());
    }
}

mod parse_command {
    use super::*;

    #[test]
    fn test_parse_json() {
        let output = Command::new(aura_binary())
            .args(["parse", "--json"])
            .arg(examples_dir().join("simple.aura"))
            .output()
            .expect("Failed to execute aura parse");

        let stdout = String::from_utf8_lossy(&output.stdout);
        let json: serde_json::Value = serde_json::from_str(&stdout)
            .expect("Output should be valid JSON");

        // Should have capabilities and definitions
        assert!(json["capabilities"].is_array());
        assert!(json["definitions"].is_array());
    }
}

mod info_command {
    use super::*;

    #[test]
    fn test_info_json() {
        let output = Command::new(aura_binary())
            .args(["info", "--json"])
            .output()
            .expect("Failed to execute aura info");

        let stdout = String::from_utf8_lossy(&output.stdout);
        let json: serde_json::Value = serde_json::from_str(&stdout)
            .expect("Output should be valid JSON");

        assert_eq!(json["name"], "AURA");
        assert!(json["version"].as_str().is_some());
        assert!(json["capabilities"].is_array());
        assert!(json["features"].is_object());
    }
}
