//! Capability ENV para AURA
//!
//! Proporciona funciones para acceder a variables de entorno.
//! Requiere +env en el programa.
//!
//! # Example
//!
//! ```aura
//! +env
//!
//! main = {
//!     // Obtener variable de entorno
//!     let api_key = env.get("API_KEY")
//!
//!     // Con valor default
//!     let db_url = env.get("DATABASE_URL", "sqlite:./default.db")
//!
//!     print!(api_key)
//! }
//! ```

use std::env;
use std::fs;
use std::path::Path;
use crate::vm::{Value, RuntimeError};

/// Loads environment variables from a .env file if it exists.
/// This is called automatically when the program starts.
///
/// The .env file format is simple:
/// - One variable per line: KEY=value
/// - Lines starting with # are comments
/// - Empty lines are ignored
/// - Values can be optionally quoted with " or '
pub fn load_dotenv() {
    load_dotenv_from_path(Path::new(".env"));
}

/// Loads environment variables from a specific path.
/// Useful for testing or loading from a custom location.
pub fn load_dotenv_from_path(path: &Path) {
    if let Ok(contents) = fs::read_to_string(path) {
        for line in contents.lines() {
            let line = line.trim();

            // Skip empty lines and comments
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Parse KEY=value
            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim();
                let value = value.trim();

                // Remove optional quotes from value
                let value = strip_quotes(value);

                // Only set if not already defined (process env takes precedence)
                if env::var(key).is_err() {
                    // SAFETY: We're in single-threaded initialization
                    unsafe { env::set_var(key, value); }
                }
            }
        }
    }
}

/// Strips surrounding quotes (single or double) from a value.
fn strip_quotes(s: &str) -> &str {
    let bytes = s.as_bytes();
    if bytes.len() >= 2 {
        let first = bytes[0];
        let last = bytes[bytes.len() - 1];
        if (first == b'"' && last == b'"') || (first == b'\'' && last == b'\'') {
            return &s[1..s.len() - 1];
        }
    }
    s
}

/// Gets an environment variable by name.
///
/// # Arguments
/// * `name` - The name of the environment variable
///
/// # Returns
/// * `Value::String` if the variable exists
/// * `Value::Nil` if the variable does not exist
pub fn env_get(name: &str) -> Value {
    match env::var(name) {
        Ok(value) => Value::String(value),
        Err(_) => Value::Nil,
    }
}

/// Gets an environment variable by name with a default value.
///
/// # Arguments
/// * `name` - The name of the environment variable
/// * `default` - The default value to return if the variable does not exist
///
/// # Returns
/// * `Value::String` with the variable value or the default
pub fn env_get_or(name: &str, default: &Value) -> Value {
    match env::var(name) {
        Ok(value) => Value::String(value),
        Err(_) => default.clone(),
    }
}

/// Sets an environment variable.
///
/// # Arguments
/// * `name` - The name of the environment variable
/// * `value` - The value to set
///
/// # Returns
/// * `Ok(Value::Nil)` on success
pub fn env_set(name: &str, value: &str) -> Result<Value, RuntimeError> {
    // SAFETY: Environment modification during program execution
    unsafe { env::set_var(name, value); }
    Ok(Value::Nil)
}

/// Removes an environment variable.
///
/// # Arguments
/// * `name` - The name of the environment variable to remove
///
/// # Returns
/// * `Ok(Value::Nil)` on success
pub fn env_remove(name: &str) -> Result<Value, RuntimeError> {
    // SAFETY: Environment modification during program execution
    unsafe { env::remove_var(name); }
    Ok(Value::Nil)
}

/// Checks if an environment variable exists.
///
/// # Arguments
/// * `name` - The name of the environment variable
///
/// # Returns
/// * `Value::Bool(true)` if the variable exists
/// * `Value::Bool(false)` if it does not exist
pub fn env_exists(name: &str) -> Value {
    Value::Bool(env::var(name).is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    // Helper functions for tests to wrap unsafe env operations
    fn test_set_var(key: &str, value: &str) {
        unsafe { env::set_var(key, value); }
    }

    fn test_remove_var(key: &str) {
        unsafe { env::remove_var(key); }
    }

    #[test]
    fn test_env_get_existing() {
        // Set a test variable
        test_set_var("AURA_TEST_VAR", "test_value");

        let result = env_get("AURA_TEST_VAR");
        assert_eq!(result, Value::String("test_value".to_string()));

        // Cleanup
        test_remove_var("AURA_TEST_VAR");
    }

    #[test]
    fn test_env_get_non_existing() {
        // Make sure the variable doesn't exist
        test_remove_var("AURA_NON_EXISTING_VAR_12345");

        let result = env_get("AURA_NON_EXISTING_VAR_12345");
        assert_eq!(result, Value::Nil);
    }

    #[test]
    fn test_env_get_or_existing() {
        test_set_var("AURA_TEST_VAR_OR", "actual_value");

        let default = Value::String("default_value".to_string());
        let result = env_get_or("AURA_TEST_VAR_OR", &default);
        assert_eq!(result, Value::String("actual_value".to_string()));

        test_remove_var("AURA_TEST_VAR_OR");
    }

    #[test]
    fn test_env_get_or_non_existing() {
        test_remove_var("AURA_NON_EXISTING_OR_12345");

        let default = Value::String("default_value".to_string());
        let result = env_get_or("AURA_NON_EXISTING_OR_12345", &default);
        assert_eq!(result, Value::String("default_value".to_string()));
    }

    #[test]
    fn test_env_set() {
        let result = env_set("AURA_SET_TEST", "new_value");
        assert!(result.is_ok());
        assert_eq!(env::var("AURA_SET_TEST").unwrap(), "new_value");

        test_remove_var("AURA_SET_TEST");
    }

    #[test]
    fn test_env_remove() {
        test_set_var("AURA_REMOVE_TEST", "value");
        assert!(env::var("AURA_REMOVE_TEST").is_ok());

        let result = env_remove("AURA_REMOVE_TEST");
        assert!(result.is_ok());
        assert!(env::var("AURA_REMOVE_TEST").is_err());
    }

    #[test]
    fn test_env_exists() {
        test_set_var("AURA_EXISTS_TEST", "value");
        assert_eq!(env_exists("AURA_EXISTS_TEST"), Value::Bool(true));

        test_remove_var("AURA_EXISTS_TEST");
        assert_eq!(env_exists("AURA_EXISTS_TEST"), Value::Bool(false));
    }

    #[test]
    fn test_strip_quotes_double() {
        assert_eq!(strip_quotes("\"hello\""), "hello");
    }

    #[test]
    fn test_strip_quotes_single() {
        assert_eq!(strip_quotes("'hello'"), "hello");
    }

    #[test]
    fn test_strip_quotes_none() {
        assert_eq!(strip_quotes("hello"), "hello");
    }

    #[test]
    fn test_strip_quotes_mismatched() {
        assert_eq!(strip_quotes("\"hello'"), "\"hello'");
    }

    #[test]
    fn test_strip_quotes_empty() {
        assert_eq!(strip_quotes(""), "");
    }

    #[test]
    fn test_load_dotenv_from_path() {
        use std::io::Write;

        // Create a temporary .env file
        let temp_dir = std::env::temp_dir();
        let env_path = temp_dir.join("aura_test.env");

        {
            let mut file = fs::File::create(&env_path).unwrap();
            writeln!(file, "# This is a comment").unwrap();
            writeln!(file, "").unwrap();
            writeln!(file, "AURA_DOTENV_TEST_1=value1").unwrap();
            writeln!(file, "AURA_DOTENV_TEST_2=\"quoted value\"").unwrap();
            writeln!(file, "AURA_DOTENV_TEST_3='single quoted'").unwrap();
            writeln!(file, "  AURA_DOTENV_TEST_4  =  spaced  ").unwrap();
        }

        // Clear any existing values
        test_remove_var("AURA_DOTENV_TEST_1");
        test_remove_var("AURA_DOTENV_TEST_2");
        test_remove_var("AURA_DOTENV_TEST_3");
        test_remove_var("AURA_DOTENV_TEST_4");

        // Load the .env file
        load_dotenv_from_path(&env_path);

        // Check values
        assert_eq!(env::var("AURA_DOTENV_TEST_1").unwrap(), "value1");
        assert_eq!(env::var("AURA_DOTENV_TEST_2").unwrap(), "quoted value");
        assert_eq!(env::var("AURA_DOTENV_TEST_3").unwrap(), "single quoted");
        assert_eq!(env::var("AURA_DOTENV_TEST_4").unwrap(), "spaced");

        // Cleanup
        fs::remove_file(&env_path).unwrap();
        test_remove_var("AURA_DOTENV_TEST_1");
        test_remove_var("AURA_DOTENV_TEST_2");
        test_remove_var("AURA_DOTENV_TEST_3");
        test_remove_var("AURA_DOTENV_TEST_4");
    }

    #[test]
    fn test_process_env_takes_precedence() {
        use std::io::Write;

        let temp_dir = std::env::temp_dir();
        let env_path = temp_dir.join("aura_test_precedence.env");

        {
            let mut file = fs::File::create(&env_path).unwrap();
            writeln!(file, "AURA_PRECEDENCE_TEST=from_file").unwrap();
        }

        // Set process env first
        test_set_var("AURA_PRECEDENCE_TEST", "from_process");

        // Load .env file
        load_dotenv_from_path(&env_path);

        // Process env should take precedence
        assert_eq!(env::var("AURA_PRECEDENCE_TEST").unwrap(), "from_process");

        // Cleanup
        fs::remove_file(&env_path).unwrap();
        test_remove_var("AURA_PRECEDENCE_TEST");
    }
}
