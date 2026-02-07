// Request model para AURA
// Convierte HTTP Request a Value de AURA

use std::collections::HashMap;
use crate::vm::Value;

/// Request HTTP representado para AURA
#[derive(Debug, Clone)]
pub struct AuraRequest {
    pub method: String,
    pub path: String,
    pub params: HashMap<String, String>,
    pub query: HashMap<String, String>,
    pub headers: HashMap<String, String>,
    pub body: Option<Value>,
}

impl AuraRequest {
    pub fn new(method: &str, path: &str) -> Self {
        Self {
            method: method.to_string(),
            path: path.to_string(),
            params: HashMap::new(),
            query: HashMap::new(),
            headers: HashMap::new(),
            body: None,
        }
    }

    pub fn with_params(mut self, params: HashMap<String, String>) -> Self {
        self.params = params;
        self
    }

    pub fn with_query(mut self, query: HashMap<String, String>) -> Self {
        self.query = query;
        self
    }

    pub fn with_body(mut self, body: Value) -> Self {
        self.body = Some(body);
        self
    }

    /// Convierte el request a un Value de AURA (Record)
    pub fn to_value(&self) -> Value {
        let mut map = HashMap::new();

        map.insert("method".to_string(), Value::String(self.method.clone()));
        map.insert("path".to_string(), Value::String(self.path.clone()));

        // Params como record
        let params: HashMap<String, Value> = self.params.iter()
            .map(|(k, v)| (k.clone(), Value::String(v.clone())))
            .collect();
        map.insert("params".to_string(), Value::Record(params));

        // Query como record
        let query: HashMap<String, Value> = self.query.iter()
            .map(|(k, v)| (k.clone(), Value::String(v.clone())))
            .collect();
        map.insert("query".to_string(), Value::Record(query));

        // Headers como record
        let headers: HashMap<String, Value> = self.headers.iter()
            .map(|(k, v)| (k.clone(), Value::String(v.clone())))
            .collect();
        map.insert("headers".to_string(), Value::Record(headers));

        // Body
        if let Some(ref body) = self.body {
            map.insert("body".to_string(), body.clone());
        } else {
            map.insert("body".to_string(), Value::Nil);
        }

        Value::Record(map)
    }
}
