//! Capabilities de AURA
//!
//! Cada capability proporciona acceso a recursos externos
//! que requieren permisos explícitos (+http, +db, +fs, +json, +env, etc.)

pub mod db;
pub mod env;
pub mod http;
pub mod json;

pub use db::{db_connect, db_query, db_execute, db_close};
pub use env::{load_dotenv, load_dotenv_from_path, env_get, env_get_or, env_set, env_remove, env_exists};
pub use http::{http_get, http_post, http_put, http_delete};
pub use json::{json_parse, json_stringify, json_stringify_pretty};
