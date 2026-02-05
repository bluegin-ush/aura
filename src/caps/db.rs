//! Capability DB para AURA
//!
//! Proporciona funciones para acceso a bases de datos SQLite y PostgreSQL.
//! Requiere +db en el programa.
//!
//! # Example
//!
//! ```aura
//! +db
//!
//! main = {
//!     // SQLite (in-memory)
//!     let conn = db.connect!(":memory:")
//!
//!     // SQLite (file-based)
//!     let conn = db.connect!("sqlite:./data.db")
//!
//!     // PostgreSQL
//!     let conn = db.connect!("postgres://user:pass@localhost/mydb")
//!
//!     db.execute!(conn, "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT)")
//!     db.execute!(conn, "INSERT INTO users (name) VALUES (?)", ["Alice"])
//!     let rows = db.query!(conn, "SELECT * FROM users")
//!     db.close!(conn)
//!     rows
//! }
//! ```

use std::collections::HashMap;
use std::sync::{Arc, Mutex, atomic::{AtomicU64, Ordering}};
use rusqlite::{Connection as SqliteConnection, params_from_iter, types::Value as SqliteValue};
use tokio_postgres::{Client as PgClient, NoTls};
use crate::vm::{Value, RuntimeError};

/// Global handle counter for all database connections
static NEXT_HANDLE: AtomicU64 = AtomicU64::new(1);

lazy_static::lazy_static! {
    /// SQLite connection registry
    static ref SQLITE_CONNECTIONS: Mutex<HashMap<u64, Arc<Mutex<SqliteConnection>>>> = Mutex::new(HashMap::new());

    /// PostgreSQL connection registry
    static ref PG_CONNECTIONS: Mutex<HashMap<u64, Arc<tokio::sync::Mutex<PgClient>>>> = Mutex::new(HashMap::new());
}

/// Type identifier for SQLite connections
const DB_TYPE_SQLITE: &str = "db:sqlite";

/// Type identifier for PostgreSQL connections
const DB_TYPE_POSTGRES: &str = "db:postgres";

/// Detects the database type from the URL and returns the appropriate type identifier.
fn detect_db_type(url: &str) -> (&'static str, &str) {
    if url == ":memory:" {
        (DB_TYPE_SQLITE, url)
    } else if url.starts_with("sqlite:") {
        (DB_TYPE_SQLITE, &url[7..]) // Remove "sqlite:" prefix
    } else if url.starts_with("postgres://") || url.starts_with("postgresql://") {
        (DB_TYPE_POSTGRES, url)
    } else {
        // Default to SQLite for file paths
        (DB_TYPE_SQLITE, url)
    }
}

/// Connects to a database (SQLite or PostgreSQL).
///
/// # URL Formats
/// * `:memory:` - SQLite in-memory database
/// * `sqlite:path` - SQLite file database
/// * `postgres://user:pass@host/db` - PostgreSQL database
/// * `path/to/file.db` - SQLite file (default)
///
/// # Returns
/// A `Value::Native` representing the database connection handle.
///
/// # Errors
/// Returns `RuntimeError` if the connection cannot be established.
pub fn db_connect(url: &str) -> Result<Value, RuntimeError> {
    let (db_type, conn_url) = detect_db_type(url);

    match db_type {
        DB_TYPE_SQLITE => connect_sqlite(conn_url),
        DB_TYPE_POSTGRES => connect_postgres(conn_url),
        _ => Err(RuntimeError::new(format!("Unknown database type for URL: {}", url))),
    }
}

/// Connects to a SQLite database.
fn connect_sqlite(url: &str) -> Result<Value, RuntimeError> {
    let conn = if url == ":memory:" {
        SqliteConnection::open_in_memory()
    } else {
        SqliteConnection::open(url)
    };

    match conn {
        Ok(connection) => {
            let handle = NEXT_HANDLE.fetch_add(1, Ordering::SeqCst);
            let mut registry = SQLITE_CONNECTIONS.lock()
                .map_err(|e| RuntimeError::new(format!("SQLite: Failed to acquire connection registry: {}", e)))?;
            registry.insert(handle, Arc::new(Mutex::new(connection)));

            Ok(Value::Native {
                type_id: DB_TYPE_SQLITE.to_string(),
                handle,
            })
        }
        Err(e) => Err(RuntimeError::new(format!("SQLite connection error: {}", e))),
    }
}

/// Connects to a PostgreSQL database.
fn connect_postgres(url: &str) -> Result<Value, RuntimeError> {
    // Get or create a tokio runtime for blocking operations
    let runtime = get_or_create_runtime()?;

    let handle = NEXT_HANDLE.fetch_add(1, Ordering::SeqCst);

    // Run the async connection in a blocking context
    runtime.block_on(async {
        let (client, connection) = tokio_postgres::connect(url, NoTls)
            .await
            .map_err(|e| RuntimeError::new(format!("PostgreSQL connection error: {}", e)))?;

        // Spawn the connection task to handle background communication
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("PostgreSQL connection error: {}", e);
            }
        });

        let mut registry = PG_CONNECTIONS.lock()
            .map_err(|e| RuntimeError::new(format!("PostgreSQL: Failed to acquire connection registry: {}", e)))?;
        registry.insert(handle, Arc::new(tokio::sync::Mutex::new(client)));

        Ok(Value::Native {
            type_id: DB_TYPE_POSTGRES.to_string(),
            handle,
        })
    })
}

/// Gets the current tokio runtime or creates a new one if not in an async context.
fn get_or_create_runtime() -> Result<tokio::runtime::Handle, RuntimeError> {
    // Try to get the current runtime handle
    match tokio::runtime::Handle::try_current() {
        Ok(handle) => Ok(handle),
        Err(_) => {
            // No runtime exists, create a new one
            // This is stored in a thread-local to avoid creating multiple runtimes
            thread_local! {
                static RUNTIME: std::cell::RefCell<Option<tokio::runtime::Runtime>> = const { std::cell::RefCell::new(None) };
            }

            RUNTIME.with(|rt| {
                let mut rt_ref = rt.borrow_mut();
                if rt_ref.is_none() {
                    *rt_ref = Some(
                        tokio::runtime::Builder::new_multi_thread()
                            .enable_all()
                            .build()
                            .map_err(|e| RuntimeError::new(format!("Failed to create tokio runtime: {}", e)))?
                    );
                }
                Ok(rt_ref.as_ref().unwrap().handle().clone())
            })
        }
    }
}

/// Executes a SELECT query and returns the results.
///
/// # Arguments
/// * `conn` - Database connection handle (from `db_connect`)
/// * `sql` - SQL query string
/// * `params` - Query parameters (positional, using `?` for SQLite, `$1` for PostgreSQL)
///
/// # Returns
/// A `Value::List` containing `Value::Record` for each row.
/// Each record maps column names to their values.
///
/// # Errors
/// Returns `RuntimeError` if the query fails or connection is invalid.
pub fn db_query(conn: &Value, sql: &str, params: &[Value]) -> Result<Value, RuntimeError> {
    match conn {
        Value::Native { type_id, handle } if type_id == DB_TYPE_SQLITE => {
            query_sqlite(*handle, sql, params)
        }
        Value::Native { type_id, handle } if type_id == DB_TYPE_POSTGRES => {
            query_postgres(*handle, sql, params)
        }
        Value::Native { type_id, .. } => {
            Err(RuntimeError::new(format!(
                "Expected database connection, got native handle of type '{}'",
                type_id
            )))
        }
        _ => {
            Err(RuntimeError::new(format!(
                "Expected database connection, got {:?}",
                conn
            )))
        }
    }
}

/// Executes a SQLite query.
fn query_sqlite(handle: u64, sql: &str, params: &[Value]) -> Result<Value, RuntimeError> {
    let connection = get_sqlite_connection(handle)?;
    let conn_guard = connection.lock()
        .map_err(|e| RuntimeError::new(format!("SQLite: Failed to acquire connection lock: {}", e)))?;

    let sql_params = convert_params_sqlite(params)?;
    let param_refs: Vec<&dyn rusqlite::ToSql> = sql_params
        .iter()
        .map(|v| v as &dyn rusqlite::ToSql)
        .collect();

    let mut stmt = conn_guard.prepare(sql)
        .map_err(|e| RuntimeError::new(format!("SQLite prepare error: {}", e)))?;

    let column_names: Vec<String> = stmt.column_names()
        .iter()
        .map(|s| s.to_string())
        .collect();

    let rows = stmt.query_map(params_from_iter(param_refs.iter()), |row| {
        let mut record = HashMap::new();
        for (i, name) in column_names.iter().enumerate() {
            let value = row.get_ref(i)?;
            record.insert(name.clone(), sqlite_value_to_aura(value));
        }
        Ok(Value::Record(record))
    }).map_err(|e| RuntimeError::new(format!("SQLite query error: {}", e)))?;

    let mut results = Vec::new();
    for row_result in rows {
        match row_result {
            Ok(row) => results.push(row),
            Err(e) => return Err(RuntimeError::new(format!("SQLite row error: {}", e))),
        }
    }

    Ok(Value::List(results))
}

/// Executes a PostgreSQL query.
fn query_postgres(handle: u64, sql: &str, params: &[Value]) -> Result<Value, RuntimeError> {
    let runtime = get_or_create_runtime()?;
    let client = get_postgres_connection(handle)?;

    runtime.block_on(async {
        let client_guard = client.lock().await;

        // Convert parameters to PostgreSQL types
        let pg_params = convert_params_postgres(params)?;
        let param_refs: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = pg_params
            .iter()
            .map(|v| v.as_ref() as &(dyn tokio_postgres::types::ToSql + Sync))
            .collect();

        let rows = client_guard.query(sql, &param_refs)
            .await
            .map_err(|e| RuntimeError::new(format!("PostgreSQL query error: {}", e)))?;

        let mut results = Vec::new();
        for row in rows {
            let mut record = HashMap::new();
            for (i, column) in row.columns().iter().enumerate() {
                let value = pg_value_to_aura(&row, i)?;
                record.insert(column.name().to_string(), value);
            }
            results.push(Value::Record(record));
        }

        Ok(Value::List(results))
    })
}

/// Executes an INSERT, UPDATE, DELETE, or DDL statement.
///
/// # Arguments
/// * `conn` - Database connection handle (from `db_connect`)
/// * `sql` - SQL statement
/// * `params` - Statement parameters (positional, using `?` for SQLite, `$1` for PostgreSQL)
///
/// # Returns
/// A `Value::Record` with:
/// - `rows_affected`: Number of rows modified (for INSERT/UPDATE/DELETE)
/// - `last_insert_id`: Last inserted row ID (for INSERT, SQLite only)
///
/// # Errors
/// Returns `RuntimeError` if the statement fails or connection is invalid.
pub fn db_execute(conn: &Value, sql: &str, params: &[Value]) -> Result<Value, RuntimeError> {
    match conn {
        Value::Native { type_id, handle } if type_id == DB_TYPE_SQLITE => {
            execute_sqlite(*handle, sql, params)
        }
        Value::Native { type_id, handle } if type_id == DB_TYPE_POSTGRES => {
            execute_postgres(*handle, sql, params)
        }
        Value::Native { type_id, .. } => {
            Err(RuntimeError::new(format!(
                "Expected database connection, got native handle of type '{}'",
                type_id
            )))
        }
        _ => {
            Err(RuntimeError::new(format!(
                "Expected database connection, got {:?}",
                conn
            )))
        }
    }
}

/// Executes a SQLite statement.
fn execute_sqlite(handle: u64, sql: &str, params: &[Value]) -> Result<Value, RuntimeError> {
    let connection = get_sqlite_connection(handle)?;
    let conn_guard = connection.lock()
        .map_err(|e| RuntimeError::new(format!("SQLite: Failed to acquire connection lock: {}", e)))?;

    let sql_params = convert_params_sqlite(params)?;
    let param_refs: Vec<&dyn rusqlite::ToSql> = sql_params
        .iter()
        .map(|v| v as &dyn rusqlite::ToSql)
        .collect();

    let rows_affected = conn_guard.execute(sql, params_from_iter(param_refs.iter()))
        .map_err(|e| RuntimeError::new(format!("SQLite execute error: {}", e)))?;

    let last_insert_id = conn_guard.last_insert_rowid();

    let mut result = HashMap::new();
    result.insert("rows_affected".to_string(), Value::Int(rows_affected as i64));
    result.insert("last_insert_id".to_string(), Value::Int(last_insert_id));

    Ok(Value::Record(result))
}

/// Executes a PostgreSQL statement.
fn execute_postgres(handle: u64, sql: &str, params: &[Value]) -> Result<Value, RuntimeError> {
    let runtime = get_or_create_runtime()?;
    let client = get_postgres_connection(handle)?;

    runtime.block_on(async {
        let client_guard = client.lock().await;

        let pg_params = convert_params_postgres(params)?;
        let param_refs: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = pg_params
            .iter()
            .map(|v| v.as_ref() as &(dyn tokio_postgres::types::ToSql + Sync))
            .collect();

        let rows_affected = client_guard.execute(sql, &param_refs)
            .await
            .map_err(|e| RuntimeError::new(format!("PostgreSQL execute error: {}", e)))?;

        let mut result = HashMap::new();
        result.insert("rows_affected".to_string(), Value::Int(rows_affected as i64));
        // PostgreSQL doesn't have a simple last_insert_id like SQLite
        // Use RETURNING clause in INSERT statements instead
        result.insert("last_insert_id".to_string(), Value::Int(0));

        Ok(Value::Record(result))
    })
}

/// Closes a database connection.
///
/// # Arguments
/// * `conn` - Database connection handle (from `db_connect`)
///
/// # Returns
/// `Ok(())` if the connection was closed successfully.
///
/// # Errors
/// Returns `RuntimeError` if the connection is invalid or already closed.
pub fn db_close(conn: &Value) -> Result<(), RuntimeError> {
    match conn {
        Value::Native { type_id, handle } if type_id == DB_TYPE_SQLITE => {
            close_sqlite(*handle)
        }
        Value::Native { type_id, handle } if type_id == DB_TYPE_POSTGRES => {
            close_postgres(*handle)
        }
        Value::Native { type_id, .. } => {
            Err(RuntimeError::new(format!(
                "Expected database connection, got native handle of type '{}'",
                type_id
            )))
        }
        _ => {
            Err(RuntimeError::new(format!(
                "Expected database connection, got {:?}",
                conn
            )))
        }
    }
}

/// Closes a SQLite connection.
fn close_sqlite(handle: u64) -> Result<(), RuntimeError> {
    let mut registry = SQLITE_CONNECTIONS.lock()
        .map_err(|e| RuntimeError::new(format!("SQLite: Failed to acquire connection registry: {}", e)))?;

    if registry.remove(&handle).is_none() {
        return Err(RuntimeError::new(format!(
            "SQLite connection #{} not found or already closed",
            handle
        )));
    }

    Ok(())
}

/// Closes a PostgreSQL connection.
fn close_postgres(handle: u64) -> Result<(), RuntimeError> {
    let mut registry = PG_CONNECTIONS.lock()
        .map_err(|e| RuntimeError::new(format!("PostgreSQL: Failed to acquire connection registry: {}", e)))?;

    if registry.remove(&handle).is_none() {
        return Err(RuntimeError::new(format!(
            "PostgreSQL connection #{} not found or already closed",
            handle
        )));
    }

    Ok(())
}

/// Retrieves a SQLite connection from the registry.
fn get_sqlite_connection(handle: u64) -> Result<Arc<Mutex<SqliteConnection>>, RuntimeError> {
    let registry = SQLITE_CONNECTIONS.lock()
        .map_err(|e| RuntimeError::new(format!("SQLite: Failed to acquire connection registry: {}", e)))?;

    registry.get(&handle)
        .cloned()
        .ok_or_else(|| RuntimeError::new(format!(
            "SQLite connection #{} not found or already closed",
            handle
        )))
}

/// Retrieves a PostgreSQL connection from the registry.
fn get_postgres_connection(handle: u64) -> Result<Arc<tokio::sync::Mutex<PgClient>>, RuntimeError> {
    let registry = PG_CONNECTIONS.lock()
        .map_err(|e| RuntimeError::new(format!("PostgreSQL: Failed to acquire connection registry: {}", e)))?;

    registry.get(&handle)
        .cloned()
        .ok_or_else(|| RuntimeError::new(format!(
            "PostgreSQL connection #{} not found or already closed",
            handle
        )))
}

// ============================================================================
// SQLite Type Conversions
// ============================================================================

/// Converts AURA values to SQLite parameters.
fn convert_params_sqlite(params: &[Value]) -> Result<Vec<SqliteValue>, RuntimeError> {
    params.iter().map(|v| aura_to_sqlite_value(v)).collect()
}

/// Converts an AURA value to a SQLite value.
fn aura_to_sqlite_value(value: &Value) -> Result<SqliteValue, RuntimeError> {
    match value {
        Value::Nil => Ok(SqliteValue::Null),
        Value::Int(n) => Ok(SqliteValue::Integer(*n)),
        Value::Float(f) => Ok(SqliteValue::Real(*f)),
        Value::String(s) => Ok(SqliteValue::Text(s.clone())),
        Value::Bool(b) => Ok(SqliteValue::Integer(if *b { 1 } else { 0 })),
        Value::List(items) => {
            // Convert list to JSON string for storage
            let json = serde_json::to_string(items)
                .map_err(|e| RuntimeError::new(format!("SQLite: Cannot convert list to SQL: {}", e)))?;
            Ok(SqliteValue::Text(json))
        }
        Value::Record(fields) => {
            // Convert record to JSON string for storage
            let json = serde_json::to_string(fields)
                .map_err(|e| RuntimeError::new(format!("SQLite: Cannot convert record to SQL: {}", e)))?;
            Ok(SqliteValue::Text(json))
        }
        Value::Function(name) => Err(RuntimeError::new(format!(
            "SQLite: Cannot use function '{}' as SQL parameter",
            name
        ))),
        Value::Native { type_id, .. } => Err(RuntimeError::new(format!(
            "SQLite: Cannot use native handle '{}' as SQL parameter",
            type_id
        ))),
    }
}

/// Converts a SQLite value reference to an AURA value.
fn sqlite_value_to_aura(value: rusqlite::types::ValueRef) -> Value {
    match value {
        rusqlite::types::ValueRef::Null => Value::Nil,
        rusqlite::types::ValueRef::Integer(n) => Value::Int(n),
        rusqlite::types::ValueRef::Real(f) => Value::Float(f),
        rusqlite::types::ValueRef::Text(s) => {
            Value::String(String::from_utf8_lossy(s).to_string())
        }
        rusqlite::types::ValueRef::Blob(b) => {
            // Convert blob to base64 string
            Value::String(format!("blob:{}", base64_encode(b)))
        }
    }
}

// ============================================================================
// PostgreSQL Type Conversions
// ============================================================================

/// A wrapper enum for PostgreSQL parameter values.
#[derive(Debug)]
enum PgParam {
    Null,
    Int(i64),
    Float(f64),
    String(String),
    Bool(bool),
}

impl tokio_postgres::types::ToSql for PgParam {
    fn to_sql(&self, ty: &tokio_postgres::types::Type, out: &mut bytes::BytesMut) -> Result<tokio_postgres::types::IsNull, Box<dyn std::error::Error + Sync + Send>> {
        match self {
            PgParam::Null => Ok(tokio_postgres::types::IsNull::Yes),
            PgParam::Int(n) => n.to_sql(ty, out),
            PgParam::Float(f) => f.to_sql(ty, out),
            PgParam::String(s) => s.to_sql(ty, out),
            PgParam::Bool(b) => b.to_sql(ty, out),
        }
    }

    fn accepts(ty: &tokio_postgres::types::Type) -> bool {
        <i64 as tokio_postgres::types::ToSql>::accepts(ty)
            || <f64 as tokio_postgres::types::ToSql>::accepts(ty)
            || <String as tokio_postgres::types::ToSql>::accepts(ty)
            || <bool as tokio_postgres::types::ToSql>::accepts(ty)
            || *ty == tokio_postgres::types::Type::TEXT
            || *ty == tokio_postgres::types::Type::VARCHAR
    }

    tokio_postgres::types::to_sql_checked!();
}

/// Converts AURA values to PostgreSQL parameters.
fn convert_params_postgres(params: &[Value]) -> Result<Vec<Box<dyn tokio_postgres::types::ToSql + Sync + Send>>, RuntimeError> {
    params.iter().map(|v| aura_to_pg_param(v)).collect()
}

/// Converts an AURA value to a PostgreSQL parameter.
fn aura_to_pg_param(value: &Value) -> Result<Box<dyn tokio_postgres::types::ToSql + Sync + Send>, RuntimeError> {
    match value {
        Value::Nil => Ok(Box::new(PgParam::Null)),
        Value::Int(n) => Ok(Box::new(PgParam::Int(*n))),
        Value::Float(f) => Ok(Box::new(PgParam::Float(*f))),
        Value::String(s) => Ok(Box::new(PgParam::String(s.clone()))),
        Value::Bool(b) => Ok(Box::new(PgParam::Bool(*b))),
        Value::List(items) => {
            // Convert list to JSON string for storage
            let json = serde_json::to_string(items)
                .map_err(|e| RuntimeError::new(format!("PostgreSQL: Cannot convert list to SQL: {}", e)))?;
            Ok(Box::new(PgParam::String(json)))
        }
        Value::Record(fields) => {
            // Convert record to JSON string for storage
            let json = serde_json::to_string(fields)
                .map_err(|e| RuntimeError::new(format!("PostgreSQL: Cannot convert record to SQL: {}", e)))?;
            Ok(Box::new(PgParam::String(json)))
        }
        Value::Function(name) => Err(RuntimeError::new(format!(
            "PostgreSQL: Cannot use function '{}' as SQL parameter",
            name
        ))),
        Value::Native { type_id, .. } => Err(RuntimeError::new(format!(
            "PostgreSQL: Cannot use native handle '{}' as SQL parameter",
            type_id
        ))),
    }
}

/// Converts a PostgreSQL row value to an AURA value.
fn pg_value_to_aura(row: &tokio_postgres::Row, idx: usize) -> Result<Value, RuntimeError> {
    let column = &row.columns()[idx];
    let type_name = column.type_().name();

    // Try to get the value based on the column type
    match type_name {
        "int2" | "int4" | "int8" => {
            if let Ok(v) = row.try_get::<_, Option<i64>>(idx) {
                return Ok(v.map(Value::Int).unwrap_or(Value::Nil));
            }
            if let Ok(v) = row.try_get::<_, Option<i32>>(idx) {
                return Ok(v.map(|n| Value::Int(n as i64)).unwrap_or(Value::Nil));
            }
            if let Ok(v) = row.try_get::<_, Option<i16>>(idx) {
                return Ok(v.map(|n| Value::Int(n as i64)).unwrap_or(Value::Nil));
            }
            Ok(Value::Nil)
        }
        "float4" | "float8" | "numeric" => {
            if let Ok(v) = row.try_get::<_, Option<f64>>(idx) {
                return Ok(v.map(Value::Float).unwrap_or(Value::Nil));
            }
            if let Ok(v) = row.try_get::<_, Option<f32>>(idx) {
                return Ok(v.map(|n| Value::Float(n as f64)).unwrap_or(Value::Nil));
            }
            Ok(Value::Nil)
        }
        "bool" => {
            if let Ok(v) = row.try_get::<_, Option<bool>>(idx) {
                return Ok(v.map(Value::Bool).unwrap_or(Value::Nil));
            }
            Ok(Value::Nil)
        }
        "text" | "varchar" | "char" | "name" | "bpchar" => {
            if let Ok(v) = row.try_get::<_, Option<String>>(idx) {
                return Ok(v.map(Value::String).unwrap_or(Value::Nil));
            }
            Ok(Value::Nil)
        }
        "json" | "jsonb" => {
            if let Ok(v) = row.try_get::<_, Option<serde_json::Value>>(idx) {
                return Ok(v.map(|json| json_to_aura(&json)).unwrap_or(Value::Nil));
            }
            Ok(Value::Nil)
        }
        "bytea" => {
            if let Ok(v) = row.try_get::<_, Option<Vec<u8>>>(idx) {
                return Ok(v.map(|b| Value::String(format!("blob:{}", base64_encode(&b)))).unwrap_or(Value::Nil));
            }
            Ok(Value::Nil)
        }
        _ => {
            // For unknown types, try to get as string
            if let Ok(v) = row.try_get::<_, Option<String>>(idx) {
                return Ok(v.map(Value::String).unwrap_or(Value::Nil));
            }
            // If all else fails, return nil
            Ok(Value::Nil)
        }
    }
}

/// Converts a serde_json::Value to an AURA Value.
fn json_to_aura(json: &serde_json::Value) -> Value {
    match json {
        serde_json::Value::Null => Value::Nil,
        serde_json::Value::Bool(b) => Value::Bool(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Value::Int(i)
            } else if let Some(f) = n.as_f64() {
                Value::Float(f)
            } else {
                Value::Nil
            }
        }
        serde_json::Value::String(s) => Value::String(s.clone()),
        serde_json::Value::Array(arr) => {
            Value::List(arr.iter().map(json_to_aura).collect())
        }
        serde_json::Value::Object(obj) => {
            let mut record = HashMap::new();
            for (k, v) in obj {
                record.insert(k.clone(), json_to_aura(v));
            }
            Value::Record(record)
        }
    }
}

// ============================================================================
// Utilities
// ============================================================================

/// Simple base64 encoding for blobs.
fn base64_encode(data: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::new();

    for chunk in data.chunks(3) {
        let b0 = chunk[0] as usize;
        let b1 = chunk.get(1).copied().unwrap_or(0) as usize;
        let b2 = chunk.get(2).copied().unwrap_or(0) as usize;

        result.push(ALPHABET[b0 >> 2] as char);
        result.push(ALPHABET[((b0 & 0x03) << 4) | (b1 >> 4)] as char);

        if chunk.len() > 1 {
            result.push(ALPHABET[((b1 & 0x0f) << 2) | (b2 >> 6)] as char);
        } else {
            result.push('=');
        }

        if chunk.len() > 2 {
            result.push(ALPHABET[b2 & 0x3f] as char);
        } else {
            result.push('=');
        }
    }

    result
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // URL Detection Tests
    // ========================================================================

    #[test]
    fn test_detect_db_type_memory() {
        let (db_type, url) = detect_db_type(":memory:");
        assert_eq!(db_type, DB_TYPE_SQLITE);
        assert_eq!(url, ":memory:");
    }

    #[test]
    fn test_detect_db_type_sqlite_prefix() {
        let (db_type, url) = detect_db_type("sqlite:./data.db");
        assert_eq!(db_type, DB_TYPE_SQLITE);
        assert_eq!(url, "./data.db");
    }

    #[test]
    fn test_detect_db_type_postgres() {
        let (db_type, url) = detect_db_type("postgres://user:pass@localhost/mydb");
        assert_eq!(db_type, DB_TYPE_POSTGRES);
        assert_eq!(url, "postgres://user:pass@localhost/mydb");
    }

    #[test]
    fn test_detect_db_type_postgresql() {
        let (db_type, url) = detect_db_type("postgresql://user:pass@localhost/mydb");
        assert_eq!(db_type, DB_TYPE_POSTGRES);
        assert_eq!(url, "postgresql://user:pass@localhost/mydb");
    }

    #[test]
    fn test_detect_db_type_file_path() {
        let (db_type, url) = detect_db_type("./data.db");
        assert_eq!(db_type, DB_TYPE_SQLITE);
        assert_eq!(url, "./data.db");
    }

    // ========================================================================
    // SQLite Tests
    // ========================================================================

    #[test]
    fn test_sqlite_connect_memory() {
        let conn = db_connect(":memory:");
        assert!(conn.is_ok());

        if let Ok(Value::Native { type_id, handle }) = conn {
            assert_eq!(type_id, DB_TYPE_SQLITE);
            assert!(handle > 0);

            // Clean up
            let _ = db_close(&Value::Native { type_id, handle });
        } else {
            panic!("Expected Native value");
        }
    }

    #[test]
    fn test_sqlite_create_table() {
        let conn = db_connect(":memory:").unwrap();

        let result = db_execute(
            &conn,
            "CREATE TABLE test (id INTEGER PRIMARY KEY, name TEXT)",
            &[],
        );
        assert!(result.is_ok());

        db_close(&conn).unwrap();
    }

    #[test]
    fn test_sqlite_insert_and_query() {
        let conn = db_connect(":memory:").unwrap();

        // Create table
        db_execute(
            &conn,
            "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT, age INTEGER)",
            &[],
        ).unwrap();

        // Insert data
        let insert_result = db_execute(
            &conn,
            "INSERT INTO users (name, age) VALUES (?, ?)",
            &[Value::String("Alice".to_string()), Value::Int(30)],
        ).unwrap();

        if let Value::Record(record) = &insert_result {
            assert_eq!(record.get("rows_affected"), Some(&Value::Int(1)));
            assert_eq!(record.get("last_insert_id"), Some(&Value::Int(1)));
        } else {
            panic!("Expected Record");
        }

        // Insert another
        db_execute(
            &conn,
            "INSERT INTO users (name, age) VALUES (?, ?)",
            &[Value::String("Bob".to_string()), Value::Int(25)],
        ).unwrap();

        // Query all
        let query_result = db_query(&conn, "SELECT * FROM users ORDER BY id", &[]).unwrap();

        if let Value::List(rows) = query_result {
            assert_eq!(rows.len(), 2);

            if let Value::Record(row1) = &rows[0] {
                assert_eq!(row1.get("id"), Some(&Value::Int(1)));
                assert_eq!(row1.get("name"), Some(&Value::String("Alice".to_string())));
                assert_eq!(row1.get("age"), Some(&Value::Int(30)));
            } else {
                panic!("Expected Record for row 1");
            }

            if let Value::Record(row2) = &rows[1] {
                assert_eq!(row2.get("id"), Some(&Value::Int(2)));
                assert_eq!(row2.get("name"), Some(&Value::String("Bob".to_string())));
                assert_eq!(row2.get("age"), Some(&Value::Int(25)));
            } else {
                panic!("Expected Record for row 2");
            }
        } else {
            panic!("Expected List");
        }

        db_close(&conn).unwrap();
    }

    #[test]
    fn test_sqlite_update() {
        let conn = db_connect(":memory:").unwrap();

        db_execute(&conn, "CREATE TABLE items (id INTEGER PRIMARY KEY, value TEXT)", &[]).unwrap();
        db_execute(&conn, "INSERT INTO items (value) VALUES (?)", &[Value::String("old".to_string())]).unwrap();

        let update_result = db_execute(
            &conn,
            "UPDATE items SET value = ? WHERE id = ?",
            &[Value::String("new".to_string()), Value::Int(1)],
        ).unwrap();

        if let Value::Record(record) = update_result {
            assert_eq!(record.get("rows_affected"), Some(&Value::Int(1)));
        } else {
            panic!("Expected Record");
        }

        let query_result = db_query(&conn, "SELECT value FROM items WHERE id = ?", &[Value::Int(1)]).unwrap();

        if let Value::List(rows) = query_result {
            assert_eq!(rows.len(), 1);
            if let Value::Record(row) = &rows[0] {
                assert_eq!(row.get("value"), Some(&Value::String("new".to_string())));
            }
        }

        db_close(&conn).unwrap();
    }

    #[test]
    fn test_sqlite_delete() {
        let conn = db_connect(":memory:").unwrap();

        db_execute(&conn, "CREATE TABLE items (id INTEGER PRIMARY KEY)", &[]).unwrap();
        db_execute(&conn, "INSERT INTO items DEFAULT VALUES", &[]).unwrap();
        db_execute(&conn, "INSERT INTO items DEFAULT VALUES", &[]).unwrap();

        let delete_result = db_execute(&conn, "DELETE FROM items WHERE id = ?", &[Value::Int(1)]).unwrap();

        if let Value::Record(record) = delete_result {
            assert_eq!(record.get("rows_affected"), Some(&Value::Int(1)));
        } else {
            panic!("Expected Record");
        }

        let query_result = db_query(&conn, "SELECT COUNT(*) as count FROM items", &[]).unwrap();

        if let Value::List(rows) = query_result {
            if let Value::Record(row) = &rows[0] {
                assert_eq!(row.get("count"), Some(&Value::Int(1)));
            }
        }

        db_close(&conn).unwrap();
    }

    #[test]
    fn test_sqlite_query_with_params() {
        let conn = db_connect(":memory:").unwrap();

        db_execute(&conn, "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT)", &[]).unwrap();
        db_execute(&conn, "INSERT INTO users (name) VALUES (?)", &[Value::String("Alice".to_string())]).unwrap();
        db_execute(&conn, "INSERT INTO users (name) VALUES (?)", &[Value::String("Bob".to_string())]).unwrap();

        let result = db_query(&conn, "SELECT * FROM users WHERE name = ?", &[Value::String("Alice".to_string())]).unwrap();

        if let Value::List(rows) = result {
            assert_eq!(rows.len(), 1);
            if let Value::Record(row) = &rows[0] {
                assert_eq!(row.get("name"), Some(&Value::String("Alice".to_string())));
            }
        } else {
            panic!("Expected List");
        }

        db_close(&conn).unwrap();
    }

    #[test]
    fn test_sqlite_null_values() {
        let conn = db_connect(":memory:").unwrap();

        db_execute(&conn, "CREATE TABLE nullable (id INTEGER PRIMARY KEY, value TEXT)", &[]).unwrap();
        db_execute(&conn, "INSERT INTO nullable (value) VALUES (?)", &[Value::Nil]).unwrap();

        let result = db_query(&conn, "SELECT value FROM nullable", &[]).unwrap();

        if let Value::List(rows) = result {
            if let Value::Record(row) = &rows[0] {
                assert_eq!(row.get("value"), Some(&Value::Nil));
            }
        }

        db_close(&conn).unwrap();
    }

    #[test]
    fn test_sqlite_float_values() {
        let conn = db_connect(":memory:").unwrap();

        db_execute(&conn, "CREATE TABLE floats (value REAL)", &[]).unwrap();
        db_execute(&conn, "INSERT INTO floats (value) VALUES (?)", &[Value::Float(3.14159)]).unwrap();

        let result = db_query(&conn, "SELECT value FROM floats", &[]).unwrap();

        if let Value::List(rows) = result {
            if let Value::Record(row) = &rows[0] {
                if let Some(Value::Float(f)) = row.get("value") {
                    assert!((f - 3.14159).abs() < 0.00001);
                } else {
                    panic!("Expected Float value");
                }
            }
        }

        db_close(&conn).unwrap();
    }

    #[test]
    fn test_sqlite_bool_values() {
        let conn = db_connect(":memory:").unwrap();

        db_execute(&conn, "CREATE TABLE bools (value INTEGER)", &[]).unwrap();
        db_execute(&conn, "INSERT INTO bools (value) VALUES (?)", &[Value::Bool(true)]).unwrap();
        db_execute(&conn, "INSERT INTO bools (value) VALUES (?)", &[Value::Bool(false)]).unwrap();

        let result = db_query(&conn, "SELECT value FROM bools ORDER BY value", &[]).unwrap();

        if let Value::List(rows) = result {
            assert_eq!(rows.len(), 2);
            // false = 0, true = 1
            if let Value::Record(row) = &rows[0] {
                assert_eq!(row.get("value"), Some(&Value::Int(0)));
            }
            if let Value::Record(row) = &rows[1] {
                assert_eq!(row.get("value"), Some(&Value::Int(1)));
            }
        }

        db_close(&conn).unwrap();
    }

    #[test]
    fn test_sqlite_close_twice_error() {
        let conn = db_connect(":memory:").unwrap();
        db_close(&conn).unwrap();

        // Second close should fail
        let result = db_close(&conn);
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("SQLite"));
    }

    #[test]
    fn test_sqlite_query_on_closed_connection_error() {
        let conn = db_connect(":memory:").unwrap();
        db_close(&conn).unwrap();

        let result = db_query(&conn, "SELECT 1", &[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("SQLite"));
    }

    #[test]
    fn test_sqlite_invalid_sql_error() {
        let conn = db_connect(":memory:").unwrap();

        let result = db_execute(&conn, "THIS IS NOT VALID SQL", &[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("SQLite"));

        db_close(&conn).unwrap();
    }

    #[test]
    fn test_wrong_connection_type() {
        let fake_conn = Value::Native {
            type_id: "other:type".to_string(),
            handle: 999,
        };

        let result = db_query(&fake_conn, "SELECT 1", &[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("Expected database connection"));
    }

    #[test]
    fn test_non_native_value_error() {
        let not_a_conn = Value::Int(42);

        let result = db_query(&not_a_conn, "SELECT 1", &[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_base64_encode() {
        assert_eq!(base64_encode(b"Hello"), "SGVsbG8=");
        assert_eq!(base64_encode(b"Hi"), "SGk=");
        assert_eq!(base64_encode(b""), "");
    }

    // ========================================================================
    // PostgreSQL Tests (require a running PostgreSQL server)
    // ========================================================================

    /// Helper function to check if PostgreSQL is available.
    fn postgres_available() -> bool {
        // Check if POSTGRES_URL environment variable is set
        std::env::var("POSTGRES_URL").is_ok()
    }

    /// Get the PostgreSQL URL from environment or use default.
    fn get_postgres_url() -> String {
        std::env::var("POSTGRES_URL")
            .unwrap_or_else(|_| "postgres://postgres:postgres@localhost/test".to_string())
    }

    #[test]
    fn test_postgres_connect() {
        if !postgres_available() {
            eprintln!("Skipping PostgreSQL test: POSTGRES_URL not set");
            return;
        }

        let url = get_postgres_url();
        let result = db_connect(&url);

        match result {
            Ok(conn) => {
                db_close(&conn).unwrap();
            }
            Err(e) => {
                eprintln!("PostgreSQL connection failed (server may not be running): {}", e.message);
            }
        }
    }

    #[test]
    fn test_postgres_create_table_and_query() {
        if !postgres_available() {
            eprintln!("Skipping PostgreSQL test: POSTGRES_URL not set");
            return;
        }

        let url = get_postgres_url();
        let conn = match db_connect(&url) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("PostgreSQL connection failed: {}", e.message);
                return;
            }
        };

        // Drop table if exists
        let _ = db_execute(&conn, "DROP TABLE IF EXISTS aura_test_users", &[]);

        // Create table
        let create_result = db_execute(
            &conn,
            "CREATE TABLE aura_test_users (id SERIAL PRIMARY KEY, name TEXT, age INTEGER)",
            &[],
        );
        assert!(create_result.is_ok(), "Failed to create table: {:?}", create_result);

        // Insert data (PostgreSQL uses $1, $2 for parameters)
        let insert_result = db_execute(
            &conn,
            "INSERT INTO aura_test_users (name, age) VALUES ($1, $2)",
            &[Value::String("Alice".to_string()), Value::Int(30)],
        );
        assert!(insert_result.is_ok(), "Failed to insert: {:?}", insert_result);

        // Query
        let query_result = db_query(&conn, "SELECT name, age FROM aura_test_users WHERE name = $1", &[Value::String("Alice".to_string())]);

        if let Ok(Value::List(rows)) = query_result {
            assert_eq!(rows.len(), 1);
            if let Value::Record(row) = &rows[0] {
                assert_eq!(row.get("name"), Some(&Value::String("Alice".to_string())));
                assert_eq!(row.get("age"), Some(&Value::Int(30)));
            }
        } else {
            panic!("Query failed: {:?}", query_result);
        }

        // Cleanup
        let _ = db_execute(&conn, "DROP TABLE aura_test_users", &[]);
        db_close(&conn).unwrap();
    }

    #[test]
    fn test_postgres_error_message_contains_postgres() {
        // Test that PostgreSQL errors contain "PostgreSQL" in the message
        let fake_conn = Value::Native {
            type_id: DB_TYPE_POSTGRES.to_string(),
            handle: 999999,
        };

        let result = db_query(&fake_conn, "SELECT 1", &[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("PostgreSQL"));
    }

    // ========================================================================
    // JSON Conversion Tests
    // ========================================================================

    #[test]
    fn test_json_to_aura() {
        let json_null = serde_json::Value::Null;
        assert_eq!(json_to_aura(&json_null), Value::Nil);

        let json_bool = serde_json::Value::Bool(true);
        assert_eq!(json_to_aura(&json_bool), Value::Bool(true));

        let json_int = serde_json::json!(42);
        assert_eq!(json_to_aura(&json_int), Value::Int(42));

        let json_float = serde_json::json!(3.14);
        assert_eq!(json_to_aura(&json_float), Value::Float(3.14));

        let json_string = serde_json::json!("hello");
        assert_eq!(json_to_aura(&json_string), Value::String("hello".to_string()));

        let json_array = serde_json::json!([1, 2, 3]);
        if let Value::List(items) = json_to_aura(&json_array) {
            assert_eq!(items.len(), 3);
            assert_eq!(items[0], Value::Int(1));
        } else {
            panic!("Expected List");
        }

        let json_object = serde_json::json!({"name": "test"});
        if let Value::Record(fields) = json_to_aura(&json_object) {
            assert_eq!(fields.get("name"), Some(&Value::String("test".to_string())));
        } else {
            panic!("Expected Record");
        }
    }
}
