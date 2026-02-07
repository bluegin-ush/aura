// HTTP Server para AURA
// Usa axum para manejar requests

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use axum::{
    Router as AxumRouter,
    routing::any,
    extract::{Path, Query, State},
    http::Method,
    body::Bytes,
};
use tokio::net::TcpListener;

use crate::vm::{VM, Value};
use crate::parser::{Program, FuncDef};
use super::router::{Router, Route};
use super::request::AuraRequest;
use super::response::AuraResponse;

/// Estado compartido del servidor
pub struct ServerState {
    pub router: Router,
    pub vm: Mutex<VM>,
    pub program: Program,
}

/// Inicia el servidor HTTP
pub async fn start_server(
    port: u16,
    routes: Vec<Route>,
    program: Program,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut router = Router::new();
    for route in routes {
        println!("  {} {}", route.method, route.path);
        router.add_route(route);
    }

    let mut vm = VM::new();
    vm.load(&program);

    let state = Arc::new(ServerState {
        router,
        vm: Mutex::new(vm),
        program,
    });

    let app = AxumRouter::new()
        .route("/*path", any(handle_request))
        .route("/", any(handle_request))
        .with_state(state);

    let addr = format!("0.0.0.0:{}", port);
    println!("AURA Server listening on http://{}", addr);

    let listener = TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

/// Handler principal que procesa todos los requests
async fn handle_request(
    State(state): State<Arc<ServerState>>,
    method: Method,
    Path(path): Path<String>,
    Query(query): Query<HashMap<String, String>>,
    body: Bytes,
) -> AuraResponse {
    let path = format!("/{}", path);
    let method_str = method.as_str();

    // Handle CORS preflight
    if method == Method::OPTIONS {
        return AuraResponse::ok(Value::Nil);
    }

    // Buscar ruta que coincida
    let route_match = state.router.find_route(method_str, &path);

    match route_match {
        Some((route, params)) => {
            // Construir request
            let mut request = AuraRequest::new(method_str, &path)
                .with_params(params.clone())
                .with_query(query);

            // Parsear body como JSON si hay contenido
            if !body.is_empty() {
                if let Ok(json_str) = std::str::from_utf8(&body) {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(json_str) {
                        request = request.with_body(json_to_value(json));
                    }
                }
            }

            // Ejecutar handler
            execute_handler(&state, &route.handler_name, request, params)
        }
        None => {
            AuraResponse::not_found(&format!("Route not found: {} {}", method_str, path))
        }
    }
}

/// Ejecuta un handler de AURA
fn execute_handler(
    state: &ServerState,
    handler_name: &str,
    request: AuraRequest,
    params: HashMap<String, String>,
) -> AuraResponse {
    let mut vm = state.vm.lock().unwrap();

    // Buscar la función handler para saber los parámetros
    let func = match find_handler(&state.program, handler_name) {
        Some(f) => f,
        None => return AuraResponse::error(&format!("Handler not found: {}", handler_name)),
    };

    // Construir argumentos basados en la firma de la función
    let mut args: Vec<Value> = Vec::new();

    for param in &func.params {
        if param.name == "req" {
            // El parámetro 'req' recibe el request completo
            args.push(request.to_value());
        } else if let Some(value) = params.get(&param.name) {
            // Parámetro de ruta (ej: :id)
            // Intentar parsear como entero, sino dejarlo como string
            if let Ok(n) = value.parse::<i64>() {
                args.push(Value::Int(n));
            } else {
                args.push(Value::String(value.clone()));
            }
        } else if let Some(Value::Record(body)) = request.body.as_ref() {
            // Buscar en el body del request
            if let Some(val) = body.get(&param.name) {
                args.push(val.clone());
            } else {
                args.push(Value::Nil);
            }
        } else {
            args.push(Value::Nil);
        }
    }

    // Llamar a la función con los argumentos
    let result = vm.call_by_name(handler_name, args);

    match result {
        Ok(value) => AuraResponse::from_value(value),
        Err(e) => AuraResponse::error(&e.message),
    }
}

/// Busca un handler por nombre en el programa
fn find_handler(program: &Program, name: &str) -> Option<FuncDef> {
    for def in &program.definitions {
        if let crate::parser::Definition::FuncDef(func) = def {
            if func.name == name {
                return Some(func.clone());
            }
        }
    }
    None
}

/// Convierte serde_json::Value a Value de AURA
fn json_to_value(json: serde_json::Value) -> Value {
    match json {
        serde_json::Value::Null => Value::Nil,
        serde_json::Value::Bool(b) => Value::Bool(b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Value::Int(i)
            } else if let Some(f) = n.as_f64() {
                Value::Float(f)
            } else {
                Value::Nil
            }
        }
        serde_json::Value::String(s) => Value::String(s),
        serde_json::Value::Array(arr) => {
            Value::List(arr.into_iter().map(json_to_value).collect())
        }
        serde_json::Value::Object(obj) => {
            let mut map = HashMap::new();
            for (k, v) in obj {
                map.insert(k, json_to_value(v));
            }
            Value::Record(map)
        }
    }
}
