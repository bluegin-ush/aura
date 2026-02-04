//! Capability DB para AURA
//!
//! Proporciona funciones para acceso a bases de datos SQLite.
//! Requiere +db en el programa.
//!
//! # Example
//!
//! ```aura
//! +db
//!
//! main = {
//!     let conn = db.connect!(":memory:")
//!     db.execute!(conn, "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT)")
//!     db.execute!(conn, "INSERT INTO users (name) VALUES (?)", ["Alice"])
//!     let rows = db.query!(conn, "SELECT * FROM users")
//!     db.close!(conn)
//!     rows
//! }
//! ```

use std::collections::HashMap;
use std::sync::{Arc, Mutex, atomic::{AtomicU64, Ordering}};
use rusqlite::{Connection, params_from_iter, types::Value as SqlValue};
use crate::vm::{Value, RuntimeError};

/// Global connection registry
/// Maps handle IDs to SQLite connections
static NEXT_HANDLE: AtomicU64 = AtomicU64::new(1);

lazy_static::lazy_static! {
    static ref CONNECTIONS: Mutex<HashMap<u64, Arc<Mutex<Connection>>>> = Mutex::new(HashMap::new());
}

/// Type identifier for database connections
const DB_TYPE_ID: &str = "db:sqlite";

/// Connects to a SQLite database.
///
/// # Arguments
/// * `url` - Database URL. Use `:memory:` for in-memory database,
///           or a file path for persistent storage.
///
/// # Returns
/// A `Value::Native` representing the database connection handle.
///
/// # Errors
/// Returns `RuntimeError` if the connection cannot be established.
pub fn db_connect(url: &str) -> Result<Value, RuntimeError> {
    let conn = if url == ":memory:" {
        Connection::open_in_memory()
    } else {
        Connection::open(url)
    };

    match conn {
        Ok(connection) => {
            let handle = NEXT_HANDLE.fetch_add(1, Ordering::SeqCst);
            let mut registry = CONNECTIONS.lock()
                .map_err(|e| RuntimeError::new(format!("Failed to acquire connection registry: {}", e)))?;
            registry.insert(handle, Arc::new(Mutex::new(connection)));

            Ok(Value::Native {
                type_id: DB_TYPE_ID.to_string(),
                handle,
            })
        }
        Err(e) => Err(RuntimeError::new(format!("Database connection error: {}", e))),
    }
}

/// Executes a SELECT query and returns the results.
///
/// # Arguments
/// * `conn` - Database connection handle (from `db_connect`)
/// * `sql` - SQL query string
/// * `params` - Query parameters (positional, using `?` placeholders)
///
/// # Returns
/// A `Value::List` containing `Value::Record` for each row.
/// Each record maps column names to their values.
///
/// # Errors
/// Returns `RuntimeError` if the query fails or connection is invalid.
pub fn db_query(conn: &Value, sql: &str, params: &[Value]) -> Result<Value, RuntimeError> {
    let connection = get_connection(conn)?;
    let conn_guard = connection.lock()
        .map_err(|e| RuntimeError::new(format!("Failed to acquire connection lock: {}", e)))?;

    let sql_params = convert_params(params)?;
    let param_refs: Vec<&dyn rusqlite::ToSql> = sql_params
        .iter()
        .map(|v| v as &dyn rusqlite::ToSql)
        .collect();

    let mut stmt = conn_guard.prepare(sql)
        .map_err(|e| RuntimeError::new(format!("SQL prepare error: {}", e)))?;

    let column_names: Vec<String> = stmt.column_names()
        .iter()
        .map(|s| s.to_string())
        .collect();

    let rows = stmt.query_map(params_from_iter(param_refs.iter()), |row| {
        let mut record = HashMap::new();
        for (i, name) in column_names.iter().enumerate() {
            let value = row.get_ref(i)?;
            record.insert(name.clone(), sql_value_to_aura(value));
        }
        Ok(Value::Record(record))
    }).map_err(|e| RuntimeError::new(format!("SQL query error: {}", e)))?;

    let mut results = Vec::new();
    for row_result in rows {
        match row_result {
            Ok(row) => results.push(row),
            Err(e) => return Err(RuntimeError::new(format!("SQL row error: {}", e))),
        }
    }

    Ok(Value::List(results))
}

/// Executes an INSERT, UPDATE, DELETE, or DDL statement.
///
/// # Arguments
/// * `conn` - Database connection handle (from `db_connect`)
/// * `sql` - SQL statement
/// * `params` - Statement parameters (positional, using `?` placeholders)
///
/// # Returns
/// A `Value::Record` with:
/// - `rows_affected`: Number of rows modified (for INSERT/UPDATE/DELETE)
/// - `last_insert_id`: Last inserted row ID (for INSERT)
///
/// # Errors
/// Returns `RuntimeError` if the statement fails or connection is invalid.
pub fn db_execute(conn: &Value, sql: &str, params: &[Value]) -> Result<Value, RuntimeError> {
    let connection = get_connection(conn)?;
    let conn_guard = connection.lock()
        .map_err(|e| RuntimeError::new(format!("Failed to acquire connection lock: {}", e)))?;

    let sql_params = convert_params(params)?;
    let param_refs: Vec<&dyn rusqlite::ToSql> = sql_params
        .iter()
        .map(|v| v as &dyn rusqlite::ToSql)
        .collect();

    let rows_affected = conn_guard.execute(sql, params_from_iter(param_refs.iter()))
        .map_err(|e| RuntimeError::new(format!("SQL execute error: {}", e)))?;

    let last_insert_id = conn_guard.last_insert_rowid();

    let mut result = HashMap::new();
    result.insert("rows_affected".to_string(), Value::Int(rows_affected as i64));
    result.insert("last_insert_id".to_string(), Value::Int(last_insert_id));

    Ok(Value::Record(result))
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
    let handle = match conn {
        Value::Native { type_id, handle } if type_id == DB_TYPE_ID => *handle,
        Value::Native { type_id, .. } => {
            return Err(RuntimeError::new(format!(
                "Expected database connection, got native handle of type '{}'",
                type_id
            )));
        }
        _ => {
            return Err(RuntimeError::new(format!(
                "Expected database connection, got {:?}",
                conn
            )));
        }
    };

    let mut registry = CONNECTIONS.lock()
        .map_err(|e| RuntimeError::new(format!("Failed to acquire connection registry: {}", e)))?;

    if registry.remove(&handle).is_none() {
        return Err(RuntimeError::new(format!(
            "Database connection #{} not found or already closed",
            handle
        )));
    }

    Ok(())
}

/// Retrieves a connection from the registry.
fn get_connection(conn: &Value) -> Result<Arc<Mutex<Connection>>, RuntimeError> {
    let handle = match conn {
        Value::Native { type_id, handle } if type_id == DB_TYPE_ID => *handle,
        Value::Native { type_id, .. } => {
            return Err(RuntimeError::new(format!(
                "Expected database connection, got native handle of type '{}'",
                type_id
            )));
        }
        _ => {
            return Err(RuntimeError::new(format!(
                "Expected database connection, got {:?}",
                conn
            )));
        }
    };

    let registry = CONNECTIONS.lock()
        .map_err(|e| RuntimeError::new(format!("Failed to acquire connection registry: {}", e)))?;

    registry.get(&handle)
        .cloned()
        .ok_or_else(|| RuntimeError::new(format!(
            "Database connection #{} not found or already closed",
            handle
        )))
}

/// Converts AURA values to SQLite parameters.
fn convert_params(params: &[Value]) -> Result<Vec<SqlValue>, RuntimeError> {
    params.iter().map(|v| aura_to_sql_value(v)).collect()
}

/// Converts an AURA value to a SQLite value.
fn aura_to_sql_value(value: &Value) -> Result<SqlValue, RuntimeError> {
    match value {
        Value::Nil => Ok(SqlValue::Null),
        Value::Int(n) => Ok(SqlValue::Integer(*n)),
        Value::Float(f) => Ok(SqlValue::Real(*f)),
        Value::String(s) => Ok(SqlValue::Text(s.clone())),
        Value::Bool(b) => Ok(SqlValue::Integer(if *b { 1 } else { 0 })),
        Value::List(items) => {
            // Convert list to JSON string for storage
            let json = serde_json::to_string(items)
                .map_err(|e| RuntimeError::new(format!("Cannot convert list to SQL: {}", e)))?;
            Ok(SqlValue::Text(json))
        }
        Value::Record(fields) => {
            // Convert record to JSON string for storage
            let json = serde_json::to_string(fields)
                .map_err(|e| RuntimeError::new(format!("Cannot convert record to SQL: {}", e)))?;
            Ok(SqlValue::Text(json))
        }
        Value::Function(name) => Err(RuntimeError::new(format!(
            "Cannot use function '{}' as SQL parameter",
            name
        ))),
        Value::Native { type_id, .. } => Err(RuntimeError::new(format!(
            "Cannot use native handle '{}' as SQL parameter",
            type_id
        ))),
    }
}

/// Converts a SQLite value reference to an AURA value.
fn sql_value_to_aura(value: rusqlite::types::ValueRef) -> Value {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_db_connect_memory() {
        let conn = db_connect(":memory:");
        assert!(conn.is_ok());

        if let Ok(Value::Native { type_id, handle }) = conn {
            assert_eq!(type_id, DB_TYPE_ID);
            assert!(handle > 0);

            // Clean up
            let _ = db_close(&Value::Native { type_id, handle });
        } else {
            panic!("Expected Native value");
        }
    }

    #[test]
    fn test_db_create_table() {
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
    fn test_db_insert_and_query() {
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
    fn test_db_update() {
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
    fn test_db_delete() {
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
    fn test_db_query_with_params() {
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
    fn test_db_null_values() {
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
    fn test_db_float_values() {
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
    fn test_db_bool_values() {
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
    fn test_db_close_twice_error() {
        let conn = db_connect(":memory:").unwrap();
        db_close(&conn).unwrap();

        // Second close should fail
        let result = db_close(&conn);
        assert!(result.is_err());
    }

    #[test]
    fn test_db_query_on_closed_connection_error() {
        let conn = db_connect(":memory:").unwrap();
        db_close(&conn).unwrap();

        let result = db_query(&conn, "SELECT 1", &[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_db_invalid_sql_error() {
        let conn = db_connect(":memory:").unwrap();

        let result = db_execute(&conn, "THIS IS NOT VALID SQL", &[]);
        assert!(result.is_err());

        db_close(&conn).unwrap();
    }

    #[test]
    fn test_db_wrong_connection_type() {
        let fake_conn = Value::Native {
            type_id: "other:type".to_string(),
            handle: 999,
        };

        let result = db_query(&fake_conn, "SELECT 1", &[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("Expected database connection"));
    }

    #[test]
    fn test_db_non_native_value_error() {
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
}
