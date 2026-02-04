//! Capabilities de AURA
//!
//! Cada capability proporciona acceso a recursos externos
//! que requieren permisos explícitos (+http, +db, +fs, +json, etc.)

pub mod http;
pub mod json;

pub use http::{http_get, http_post, http_put, http_delete};
pub use json::{json_parse, json_stringify, json_stringify_pretty};
