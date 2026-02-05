//! CLI JSON output types for agent-friendly structured output.
//!
//! This module provides consistent JSON output format for all AURA CLI commands.
//! Designed for AI agents that need to parse command results programmatically.

use serde::{Deserialize, Serialize};

/// Location information for errors and warnings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonLocation {
    pub line: usize,
    pub col: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_col: Option<usize>,
}

impl JsonLocation {
    pub fn new(line: usize, col: usize) -> Self {
        Self {
            line,
            col,
            end_col: None,
        }
    }

    pub fn with_end(line: usize, col: usize, end_col: usize) -> Self {
        Self {
            line,
            col,
            end_col: Some(end_col),
        }
    }
}

/// Error information with code, message, location, and optional suggestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonError {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<JsonLocation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
}

impl JsonError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            location: None,
            suggestion: None,
        }
    }

    pub fn with_location(mut self, location: JsonLocation) -> Self {
        self.location = Some(location);
        self
    }

    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }

    /// Create from a lexer error
    pub fn from_lex_error(err: &crate::lexer::LexError, source: &str) -> Self {
        let location = span_to_location(&err.span, source);
        Self::new("E001", &err.message).with_location(location)
    }

    /// Create from a parser error
    pub fn from_parse_error(err: &crate::parser::ParseError, source: &str) -> Self {
        let location = span_to_location(&err.span, source);
        Self::new("E101", &err.message).with_location(location)
    }

    /// Create from a type error
    pub fn from_type_error(err: &crate::types::TypeError, source: &str) -> Self {
        let mut error = Self::new("E201", &err.message);
        if let Some(ref span) = err.span {
            error = error.with_location(span_to_location(span, source));
        }
        if let Some(ref suggestion) = err.suggestion {
            error = error.with_suggestion(suggestion);
        }
        error
    }

    /// Create from a runtime error
    pub fn from_runtime_error(err: &crate::vm::RuntimeError) -> Self {
        Self::new("E401", &err.message)
    }

    /// Create for file read errors
    pub fn file_error(message: impl Into<String>) -> Self {
        Self::new("E501", message)
    }
}

/// Convert a span to a JSON location
fn span_to_location(span: &crate::lexer::Span, source: &str) -> JsonLocation {
    let before = &source[..span.start.min(source.len())];
    let line = before.lines().count().max(1);
    let last_newline = before.rfind('\n').map(|i| i + 1).unwrap_or(0);
    let col = span.start.saturating_sub(last_newline) + 1;
    let end_col = if span.end > span.start {
        Some(col + (span.end - span.start))
    } else {
        None
    };

    if let Some(end) = end_col {
        JsonLocation::with_end(line, col, end)
    } else {
        JsonLocation::new(line, col)
    }
}

/// Result of `aura check` command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckResult {
    pub success: bool,
    pub file: String,
    pub errors: Vec<JsonError>,
    pub warnings: Vec<JsonError>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stats: Option<CheckStats>,
}

/// Statistics about the checked program
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckStats {
    pub capabilities: usize,
    pub definitions: usize,
}

impl CheckResult {
    pub fn success(file: impl Into<String>, capabilities: usize, definitions: usize) -> Self {
        Self {
            success: true,
            file: file.into(),
            errors: Vec::new(),
            warnings: Vec::new(),
            stats: Some(CheckStats {
                capabilities,
                definitions,
            }),
        }
    }

    pub fn failure(file: impl Into<String>, errors: Vec<JsonError>) -> Self {
        Self {
            success: false,
            file: file.into(),
            errors,
            warnings: Vec::new(),
            stats: None,
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }
}

/// Result of `aura run` command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunResult {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "type")]
    pub result_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonError>,
}

impl RunResult {
    pub fn success(result: serde_json::Value, result_type: impl Into<String>, duration_ms: u64) -> Self {
        Self {
            success: true,
            result: Some(result),
            result_type: Some(result_type.into()),
            duration_ms: Some(duration_ms),
            error: None,
        }
    }

    pub fn failure(error: JsonError) -> Self {
        Self {
            success: false,
            result: None,
            result_type: None,
            duration_ms: None,
            error: Some(error),
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }
}

/// Convert a VM Value to JSON value and type string
pub fn value_to_json(value: &crate::vm::Value) -> (serde_json::Value, String) {
    use crate::vm::Value;
    match value {
        Value::Nil => (serde_json::Value::Null, "Nil".to_string()),
        Value::Int(n) => (serde_json::json!(n), "Int".to_string()),
        Value::Float(n) => (serde_json::json!(n), "Float".to_string()),
        Value::String(s) => (serde_json::json!(s), "String".to_string()),
        Value::Bool(b) => (serde_json::json!(b), "Bool".to_string()),
        Value::List(items) => {
            let json_items: Vec<serde_json::Value> = items
                .iter()
                .map(|v| value_to_json(v).0)
                .collect();
            (serde_json::json!(json_items), "List".to_string())
        }
        Value::Record(fields) => {
            let json_fields: serde_json::Map<String, serde_json::Value> = fields
                .iter()
                .map(|(k, v)| (k.clone(), value_to_json(v).0))
                .collect();
            (serde_json::Value::Object(json_fields), "Record".to_string())
        }
        Value::Function(name) => (serde_json::json!(format!("<fn {}>", name)), "Function".to_string()),
        Value::Native { type_id, handle } => {
            (serde_json::json!(format!("<{} #{}>", type_id, handle)), type_id.clone())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_result_success() {
        let result = CheckResult::success("test.aura", 2, 3);
        let json = result.to_json();
        assert!(json.contains("\"success\": true"));
        assert!(json.contains("\"file\": \"test.aura\""));
        assert!(json.contains("\"capabilities\": 2"));
    }

    #[test]
    fn test_check_result_failure() {
        let errors = vec![
            JsonError::new("E201", "Variable 'x' not defined")
                .with_location(JsonLocation::new(5, 3))
                .with_suggestion("Declare x before use"),
        ];
        let result = CheckResult::failure("test.aura", errors);
        let json = result.to_json();
        assert!(json.contains("\"success\": false"));
        assert!(json.contains("\"code\": \"E201\""));
        assert!(json.contains("\"line\": 5"));
    }

    #[test]
    fn test_run_result_success() {
        let result = RunResult::success(serde_json::json!("Hello World"), "String", 12);
        let json = result.to_json();
        assert!(json.contains("\"success\": true"));
        assert!(json.contains("\"result\": \"Hello World\""));
        assert!(json.contains("\"type\": \"String\""));
        assert!(json.contains("\"duration_ms\": 12"));
    }

    #[test]
    fn test_run_result_failure() {
        let error = JsonError::new("E401", "Runtime error: division by zero")
            .with_location(JsonLocation::new(10, 1));
        let result = RunResult::failure(error);
        let json = result.to_json();
        assert!(json.contains("\"success\": false"));
        assert!(json.contains("\"code\": \"E401\""));
    }

    #[test]
    fn test_value_to_json() {
        use crate::vm::Value;

        let (json, ty) = value_to_json(&Value::Int(42));
        assert_eq!(json, serde_json::json!(42));
        assert_eq!(ty, "Int");

        let (json, ty) = value_to_json(&Value::String("hello".to_string()));
        assert_eq!(json, serde_json::json!("hello"));
        assert_eq!(ty, "String");

        let (json, ty) = value_to_json(&Value::Bool(true));
        assert_eq!(json, serde_json::json!(true));
        assert_eq!(ty, "Bool");
    }
}
