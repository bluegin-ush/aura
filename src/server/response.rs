// Response model para AURA
// Convierte Value de AURA a HTTP Response

use std::collections::HashMap;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use crate::vm::Value;

/// Response HTTP desde AURA
#[derive(Debug, Clone)]
pub struct AuraResponse {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: Value,
}

impl AuraResponse {
    pub fn new(status: u16, body: Value) -> Self {
        Self {
            status,
            headers: HashMap::new(),
            body,
        }
    }

    pub fn ok(body: Value) -> Self {
        Self::new(200, body)
    }

    pub fn created(body: Value) -> Self {
        Self::new(201, body)
    }

    pub fn not_found(message: &str) -> Self {
        let mut map = HashMap::new();
        map.insert("error".to_string(), Value::String(message.to_string()));
        Self::new(404, Value::Record(map))
    }

    pub fn error(message: &str) -> Self {
        let mut map = HashMap::new();
        map.insert("error".to_string(), Value::String(message.to_string()));
        Self::new(500, Value::Record(map))
    }

    /// Convierte un Value de AURA a AuraResponse
    /// Si es un record con {status: Int, body}, usa esa estructura
    /// Si no, usa el valor completo como body con status 200
    pub fn from_value(value: Value) -> Self {
        match &value {
            Value::Record(map) => {
                // Verificar si es un response estructurado (tiene status como Int y body)
                let has_status_int = matches!(map.get("status"), Some(Value::Int(_)));
                let has_body = map.contains_key("body");

                if has_status_int && has_body {
                    // Es un response estructurado
                    let status = match map.get("status") {
                        Some(Value::Int(n)) => *n as u16,
                        _ => 200,
                    };

                    let body = map.get("body").cloned().unwrap_or(Value::Nil);

                    let headers = match map.get("headers") {
                        Some(Value::Record(h)) => {
                            h.iter()
                                .filter_map(|(k, v)| {
                                    if let Value::String(s) = v {
                                        Some((k.clone(), s.clone()))
                                    } else {
                                        None
                                    }
                                })
                                .collect()
                        }
                        _ => HashMap::new(),
                    };

                    Self { status, headers, body }
                } else {
                    // El record completo es el body
                    Self::ok(value)
                }
            }
            // Si no es un record, asumir 200 OK con el valor como body
            _ => Self::ok(value),
        }
    }
}

impl IntoResponse for AuraResponse {
    fn into_response(self) -> Response {
        let status = StatusCode::from_u16(self.status)
            .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

        let json_body = value_to_json(&self.body);

        // CORS headers
        let cors_headers = [
            (axum::http::header::ACCESS_CONTROL_ALLOW_ORIGIN, "*"),
            (axum::http::header::ACCESS_CONTROL_ALLOW_METHODS, "GET, POST, PUT, DELETE, OPTIONS"),
            (axum::http::header::ACCESS_CONTROL_ALLOW_HEADERS, "Content-Type"),
        ];

        (status, cors_headers, Json(json_body)).into_response()
    }
}

/// Convierte Value de AURA a serde_json::Value
fn value_to_json(value: &Value) -> serde_json::Value {
    match value {
        Value::Nil => serde_json::Value::Null,
        Value::Bool(b) => serde_json::Value::Bool(*b),
        Value::Int(n) => serde_json::Value::Number((*n).into()),
        Value::Float(f) => {
            serde_json::Number::from_f64(*f)
                .map(serde_json::Value::Number)
                .unwrap_or(serde_json::Value::Null)
        }
        Value::String(s) => serde_json::Value::String(s.clone()),
        Value::List(l) => {
            serde_json::Value::Array(l.iter().map(value_to_json).collect())
        }
        Value::Record(r) => {
            let obj: serde_json::Map<String, serde_json::Value> = r.iter()
                .map(|(k, v)| (k.clone(), value_to_json(v)))
                .collect();
            serde_json::Value::Object(obj)
        }
        Value::Function(name) => serde_json::Value::String(format!("<fn {}>", name)),
        Value::Native { type_id, handle } => {
            serde_json::Value::String(format!("<{} #{}>", type_id, handle))
        }
    }
}
