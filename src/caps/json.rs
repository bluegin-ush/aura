//! Capability JSON para AURA
//!
//! Proporciona funciones para parsear y serializar JSON.
//! Requiere +json en el programa.

use std::collections::HashMap;
use serde_json::{self, Value as JsonValue};
use crate::vm::{Value, RuntimeError};

/// Parsea un string JSON a un Value de AURA
pub fn json_parse(text: &str) -> Result<Value, RuntimeError> {
    match serde_json::from_str::<JsonValue>(text) {
        Ok(json) => json_to_value(json),
        Err(e) => Err(RuntimeError::new(format!("JSON parse error: {}", e))),
    }
}

/// Serializa un Value de AURA a string JSON
pub fn json_stringify(value: &Value) -> Result<String, RuntimeError> {
    let json = value_to_json(value)?;
    serde_json::to_string(&json)
        .map_err(|e| RuntimeError::new(format!("JSON stringify error: {}", e)))
}

/// Serializa un Value con formato legible (pretty print)
pub fn json_stringify_pretty(value: &Value) -> Result<String, RuntimeError> {
    let json = value_to_json(value)?;
    serde_json::to_string_pretty(&json)
        .map_err(|e| RuntimeError::new(format!("JSON stringify error: {}", e)))
}

/// Convierte un serde_json::Value a un Value de AURA
fn json_to_value(json: JsonValue) -> Result<Value, RuntimeError> {
    match json {
        JsonValue::Null => Ok(Value::Nil),
        JsonValue::Bool(b) => Ok(Value::Bool(b)),
        JsonValue::Number(n) => {
            // Intentar primero como entero, luego como flotante
            if let Some(i) = n.as_i64() {
                Ok(Value::Int(i))
            } else if let Some(f) = n.as_f64() {
                Ok(Value::Float(f))
            } else {
                Err(RuntimeError::new("JSON number out of range"))
            }
        }
        JsonValue::String(s) => Ok(Value::String(s)),
        JsonValue::Array(arr) => {
            let items: Result<Vec<Value>, RuntimeError> = arr
                .into_iter()
                .map(json_to_value)
                .collect();
            Ok(Value::List(items?))
        }
        JsonValue::Object(obj) => {
            let mut map = HashMap::new();
            for (key, val) in obj {
                map.insert(key, json_to_value(val)?);
            }
            Ok(Value::Record(map))
        }
    }
}

/// Convierte un Value de AURA a serde_json::Value
fn value_to_json(value: &Value) -> Result<JsonValue, RuntimeError> {
    match value {
        Value::Nil => Ok(JsonValue::Null),
        Value::Bool(b) => Ok(JsonValue::Bool(*b)),
        Value::Int(n) => Ok(JsonValue::Number((*n).into())),
        Value::Float(f) => {
            serde_json::Number::from_f64(*f)
                .map(JsonValue::Number)
                .ok_or_else(|| RuntimeError::new("Float value cannot be represented in JSON (NaN or Infinity)"))
        }
        Value::String(s) => Ok(JsonValue::String(s.clone())),
        Value::List(items) => {
            let arr: Result<Vec<JsonValue>, RuntimeError> = items
                .iter()
                .map(value_to_json)
                .collect();
            Ok(JsonValue::Array(arr?))
        }
        Value::Record(fields) => {
            let mut obj = serde_json::Map::new();
            for (key, val) in fields {
                obj.insert(key.clone(), value_to_json(val)?);
            }
            Ok(JsonValue::Object(obj))
        }
        Value::Function(name) => {
            // Las funciones no se pueden serializar a JSON
            Err(RuntimeError::new(format!(
                "Cannot serialize function '{}' to JSON",
                name
            )))
        }
        Value::Native { type_id, .. } => {
            // Los handles nativos no se pueden serializar a JSON
            Err(RuntimeError::new(format!(
                "Cannot serialize native handle '{}' to JSON",
                type_id
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_parse_object() {
        let json = r#"{"name": "AURA", "version": 1, "active": true}"#;
        let result = json_parse(json).unwrap();

        if let Value::Record(map) = result {
            assert_eq!(map.get("name"), Some(&Value::String("AURA".to_string())));
            assert_eq!(map.get("version"), Some(&Value::Int(1)));
            assert_eq!(map.get("active"), Some(&Value::Bool(true)));
        } else {
            panic!("Expected Record");
        }
    }

    #[test]
    fn test_json_parse_array() {
        let json = r#"[1, 2, 3, "hello", null]"#;
        let result = json_parse(json).unwrap();

        if let Value::List(items) = result {
            assert_eq!(items.len(), 5);
            assert_eq!(items[0], Value::Int(1));
            assert_eq!(items[1], Value::Int(2));
            assert_eq!(items[2], Value::Int(3));
            assert_eq!(items[3], Value::String("hello".to_string()));
            assert_eq!(items[4], Value::Nil);
        } else {
            panic!("Expected List");
        }
    }

    #[test]
    fn test_json_stringify() {
        let mut record = HashMap::new();
        record.insert("name".to_string(), Value::String("AURA".to_string()));
        record.insert("count".to_string(), Value::Int(42));

        let value = Value::Record(record);
        let json_str = json_stringify(&value).unwrap();

        // El orden de las claves puede variar, verificamos que sea JSON valido
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        assert_eq!(parsed["name"], "AURA");
        assert_eq!(parsed["count"], 42);
    }

    #[test]
    fn test_json_roundtrip() {
        let original = r#"{"items": [1, 2.5, true, null], "nested": {"a": "b"}}"#;
        let value = json_parse(original).unwrap();
        let serialized = json_stringify(&value).unwrap();
        let reparsed = json_parse(&serialized).unwrap();

        // Verificar que el valor se preserva
        assert_eq!(value, reparsed);
    }

    #[test]
    fn test_json_parse_error() {
        let invalid = r#"{"invalid": }"#;
        let result = json_parse(invalid);
        assert!(result.is_err());
    }

    #[test]
    fn test_json_stringify_function_error() {
        let func = Value::Function("my_func".to_string());
        let result = json_stringify(&func);
        assert!(result.is_err());
    }

    #[test]
    fn test_json_stringify_pretty() {
        let mut record = HashMap::new();
        record.insert("key".to_string(), Value::String("value".to_string()));

        let value = Value::Record(record);
        let pretty = json_stringify_pretty(&value).unwrap();

        // Verificar que tiene formato con saltos de linea
        assert!(pretty.contains('\n'));
    }

    #[test]
    fn test_json_stringify_native_error() {
        let native = Value::Native {
            type_id: "db:sqlite".to_string(),
            handle: 42,
        };
        let result = json_stringify(&native);
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("native handle"));
    }
}
