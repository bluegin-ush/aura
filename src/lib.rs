//! AURA - Agent-Unified Runtime Architecture
//!
//! Un lenguaje de programación diseñado para agentes de IA.
//!
//! ## Características principales
//!
//! - **Mínimos tokens**: Sintaxis ultra compacta
//! - **Cero ambigüedad**: Una sola forma de escribir cada cosa
//! - **Autocontenido**: Tipos, tests y docs en el mismo lugar
//! - **Errores JSON**: Estructurados para fácil parseo por agentes
//! - **Hot reload**: Expansión en caliente sin reiniciar
//! - **Agent bridge**: El runtime puede comunicarse con agentes IA

pub mod agent;
pub mod caps;
pub mod cli_output;
pub mod error;
pub mod lexer;
pub mod parser;
pub mod reload;
pub mod server;
pub mod types;
pub mod vm;

pub use error::{
    AuraError,
    ErrorCode,
    Errors,
    Location,
    Severity,
    Suggestion,
    format_error_pretty,
    format_errors_pretty,
};
pub use lexer::{tokenize, Token};
pub use parser::{parse, parse_expression, parse_function_def, looks_like_function_def, Program, Expr, Type, Definition, FuncDef};
pub use vm::Value;

/// Versión de AURA
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Información del runtime para agentes
pub fn runtime_info() -> serde_json::Value {
    serde_json::json!({
        "name": "AURA",
        "version": VERSION,
        "capabilities": [
            "http", "json", "db", "auth", "ws", "fs", "crypto", "time", "email"
        ],
        "features": {
            "hot_reload": true,
            "agent_bridge": true,
            "json_errors": true,
            "incremental_parsing": true
        }
    })
}
