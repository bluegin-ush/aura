//! Capability HTTP para AURA
//!
//! Proporciona funciones para hacer requests HTTP reales.
//! Requiere +http en el programa.

use std::collections::HashMap;
use crate::vm::{Value, RuntimeError};

/// Realiza un GET HTTP
pub fn http_get(url: &str, headers: Option<&HashMap<String, String>>) -> Result<Value, RuntimeError> {
    let client = reqwest::blocking::Client::new();
    let mut request = client.get(url);

    // Agregar headers si se proporcionaron
    if let Some(hdrs) = headers {
        for (key, value) in hdrs {
            request = request.header(key.as_str(), value.as_str());
        }
    }

    match request.send() {
        Ok(response) => response_to_value(response),
        Err(e) => Err(RuntimeError::new(format!("HTTP GET error: {}", e))),
    }
}

/// Realiza un POST HTTP
pub fn http_post(url: &str, body: Option<&str>, headers: Option<&HashMap<String, String>>) -> Result<Value, RuntimeError> {
    let client = reqwest::blocking::Client::new();
    let mut request = client.post(url);

    // Agregar headers si se proporcionaron
    if let Some(hdrs) = headers {
        for (key, value) in hdrs {
            request = request.header(key.as_str(), value.as_str());
        }
    }

    // Agregar body si se proporcionó
    if let Some(b) = body {
        request = request.body(b.to_string());
    }

    match request.send() {
        Ok(response) => response_to_value(response),
        Err(e) => Err(RuntimeError::new(format!("HTTP POST error: {}", e))),
    }
}

/// Realiza un PUT HTTP
pub fn http_put(url: &str, body: Option<&str>, headers: Option<&HashMap<String, String>>) -> Result<Value, RuntimeError> {
    let client = reqwest::blocking::Client::new();
    let mut request = client.put(url);

    // Agregar headers si se proporcionaron
    if let Some(hdrs) = headers {
        for (key, value) in hdrs {
            request = request.header(key.as_str(), value.as_str());
        }
    }

    // Agregar body si se proporcionó
    if let Some(b) = body {
        request = request.body(b.to_string());
    }

    match request.send() {
        Ok(response) => response_to_value(response),
        Err(e) => Err(RuntimeError::new(format!("HTTP PUT error: {}", e))),
    }
}

/// Realiza un DELETE HTTP
pub fn http_delete(url: &str, headers: Option<&HashMap<String, String>>) -> Result<Value, RuntimeError> {
    let client = reqwest::blocking::Client::new();
    let mut request = client.delete(url);

    // Agregar headers si se proporcionaron
    if let Some(hdrs) = headers {
        for (key, value) in hdrs {
            request = request.header(key.as_str(), value.as_str());
        }
    }

    match request.send() {
        Ok(response) => response_to_value(response),
        Err(e) => Err(RuntimeError::new(format!("HTTP DELETE error: {}", e))),
    }
}

/// Convierte una respuesta HTTP a un Value::Record
fn response_to_value(response: reqwest::blocking::Response) -> Result<Value, RuntimeError> {
    let status = response.status().as_u16() as i64;

    // Extraer headers
    let mut headers_map = HashMap::new();
    for (name, value) in response.headers() {
        if let Ok(v) = value.to_str() {
            headers_map.insert(
                name.to_string(),
                Value::String(v.to_string())
            );
        }
    }

    // Extraer body como string
    let body = match response.text() {
        Ok(text) => Value::String(text),
        Err(e) => return Err(RuntimeError::new(format!("Error reading response body: {}", e))),
    };

    // Construir el record de respuesta
    let mut record = HashMap::new();
    record.insert("status".to_string(), Value::Int(status));
    record.insert("headers".to_string(), Value::Record(headers_map));
    record.insert("body".to_string(), body);

    Ok(Value::Record(record))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http_get_httpbin() {
        let result = http_get("https://httpbin.org/get", None);
        assert!(result.is_ok());

        if let Ok(Value::Record(record)) = result {
            assert!(record.contains_key("status"));
            assert!(record.contains_key("headers"));
            assert!(record.contains_key("body"));

            if let Some(Value::Int(status)) = record.get("status") {
                assert_eq!(*status, 200);
            }
        }
    }

    #[test]
    fn test_http_post_httpbin() {
        let result = http_post("https://httpbin.org/post", Some("{\"test\": true}"), None);
        assert!(result.is_ok());

        if let Ok(Value::Record(record)) = result {
            if let Some(Value::Int(status)) = record.get("status") {
                assert_eq!(*status, 200);
            }
        }
    }
}
