//! Healing Memory - Memoria persistente de patrones de errores y fixes
//!
//! Este modulo permite que el sistema de healing recuerde patrones de errores
//! y sus soluciones para no repetir errores tontos y aplicar fixes conocidos
//! de forma mas rapida y consistente.
//!
//! ## Archivo de memoria
//!
//! La memoria se guarda en `.aura-memory.json` en el directorio del proyecto.
//!
//! ## Ejemplo de uso
//!
//! ```ignore
//! use aura::agent::memory::HealingMemory;
//!
//! // Cargar memoria existente
//! let mut memory = HealingMemory::load(".aura-memory.json")?;
//!
//! // Buscar patron conocido
//! if let Some(pattern) = memory.find_pattern("Variable no definida: api_url") {
//!     println!("Fix conocido: {}", pattern.fix);
//! }
//!
//! // Registrar un nuevo fix
//! memory.record_fix(
//!     "Variable no definida: api_url",
//!     "uso de http.get",
//!     "api_url = \"https://api.example.com\""
//! );
//!
//! // Guardar memoria
//! memory.save(".aura-memory.json")?;
//! ```

use std::collections::HashMap;
use std::path::Path;
use std::fs;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// Version actual del formato de memoria
pub const MEMORY_VERSION: &str = "1.0";

/// Nombre del archivo de memoria por defecto
pub const MEMORY_FILE: &str = ".aura-memory.json";

/// Memoria de healing persistente
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealingMemory {
    /// Version del formato
    pub version: String,

    /// Patrones de errores conocidos
    pub patterns: Vec<Pattern>,

    /// Valores por defecto del proyecto
    pub project_defaults: HashMap<String, String>,
}

/// Patron de error conocido
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pattern {
    /// Mensaje de error (o patron de mensaje)
    pub error: String,

    /// Contexto donde ocurrio el error
    pub context: String,

    /// Fix que se aplico
    pub fix: String,

    /// Numero de veces que se ha usado este patron
    pub count: u32,

    /// Ultima vez que se uso
    pub last_used: DateTime<Utc>,
}

/// Errores del sistema de memoria
#[derive(Debug, Clone)]
pub enum MemoryError {
    /// Error de IO
    IoError(String),
    /// Error de serializacion/deserializacion
    SerdeError(String),
    /// Version incompatible
    VersionMismatch { expected: String, found: String },
}

impl std::fmt::Display for MemoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MemoryError::IoError(msg) => write!(f, "IO error: {}", msg),
            MemoryError::SerdeError(msg) => write!(f, "Serialization error: {}", msg),
            MemoryError::VersionMismatch { expected, found } => {
                write!(f, "Version mismatch: expected {}, found {}", expected, found)
            }
        }
    }
}

impl std::error::Error for MemoryError {}

impl From<std::io::Error> for MemoryError {
    fn from(err: std::io::Error) -> Self {
        MemoryError::IoError(err.to_string())
    }
}

impl From<serde_json::Error> for MemoryError {
    fn from(err: serde_json::Error) -> Self {
        MemoryError::SerdeError(err.to_string())
    }
}

impl Default for HealingMemory {
    fn default() -> Self {
        Self::new()
    }
}

impl HealingMemory {
    /// Crea una nueva memoria vacia
    pub fn new() -> Self {
        Self {
            version: MEMORY_VERSION.to_string(),
            patterns: Vec::new(),
            project_defaults: HashMap::new(),
        }
    }

    /// Carga la memoria desde un archivo
    ///
    /// Si el archivo no existe, retorna una memoria vacia.
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, MemoryError> {
        let path = path.as_ref();

        if !path.exists() {
            return Ok(Self::new());
        }

        let content = fs::read_to_string(path)?;
        let memory: HealingMemory = serde_json::from_str(&content)?;

        // Verificar version (por ahora solo advertir, no fallar)
        if memory.version != MEMORY_VERSION {
            // En futuras versiones podriamos migrar el formato
            // Por ahora, simplemente lo cargamos
        }

        Ok(memory)
    }

    /// Guarda la memoria a un archivo
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), MemoryError> {
        let content = serde_json::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }

    /// Busca un patron que coincida con el error dado
    ///
    /// Usa coincidencia parcial: si el mensaje de error contiene el patron
    /// o el patron contiene el mensaje, se considera coincidencia.
    pub fn find_pattern(&self, error: &str) -> Option<&Pattern> {
        // Normalizar el error para comparacion
        let error_lower = error.to_lowercase();

        // Buscar coincidencia exacta primero
        if let Some(pattern) = self.patterns.iter().find(|p| p.error.to_lowercase() == error_lower) {
            return Some(pattern);
        }

        // Buscar coincidencia parcial
        self.patterns.iter().find(|p| {
            let pattern_lower = p.error.to_lowercase();
            error_lower.contains(&pattern_lower) || pattern_lower.contains(&error_lower)
        })
    }

    /// Busca un patron que coincida con el error y contexto
    pub fn find_pattern_with_context(&self, error: &str, context: &str) -> Option<&Pattern> {
        let error_lower = error.to_lowercase();
        let context_lower = context.to_lowercase();

        self.patterns.iter().find(|p| {
            let pattern_error_lower = p.error.to_lowercase();
            let pattern_context_lower = p.context.to_lowercase();

            // Coincidencia de error
            let error_match = error_lower.contains(&pattern_error_lower)
                || pattern_error_lower.contains(&error_lower);

            // Coincidencia de contexto (si existe)
            let context_match = p.context.is_empty()
                || context_lower.contains(&pattern_context_lower)
                || pattern_context_lower.contains(&context_lower);

            error_match && context_match
        })
    }

    /// Registra un fix exitoso
    ///
    /// Si ya existe un patron similar, incrementa el contador.
    /// Si no existe, crea uno nuevo.
    pub fn record_fix(&mut self, error: &str, context: &str, fix: &str) {
        let now = Utc::now();

        // Buscar patron existente
        if let Some(index) = self.patterns.iter().position(|p| {
            p.error.to_lowercase() == error.to_lowercase()
        }) {
            // Actualizar patron existente
            self.patterns[index].count += 1;
            self.patterns[index].last_used = now;
            // Actualizar el fix si es diferente (el mas reciente gana)
            if self.patterns[index].fix != fix {
                self.patterns[index].fix = fix.to_string();
            }
            // Actualizar contexto si estaba vacio
            if self.patterns[index].context.is_empty() && !context.is_empty() {
                self.patterns[index].context = context.to_string();
            }
        } else {
            // Crear nuevo patron
            self.patterns.push(Pattern {
                error: error.to_string(),
                context: context.to_string(),
                fix: fix.to_string(),
                count: 1,
                last_used: now,
            });
        }
    }

    /// Obtiene un valor por defecto del proyecto
    pub fn get_default(&self, key: &str) -> Option<&String> {
        self.project_defaults.get(key)
    }

    /// Establece un valor por defecto del proyecto
    pub fn set_default(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.project_defaults.insert(key.into(), value.into());
    }

    /// Obtiene todos los defaults del proyecto
    pub fn get_all_defaults(&self) -> &HashMap<String, String> {
        &self.project_defaults
    }

    /// Limpia todos los patrones
    pub fn clear_patterns(&mut self) {
        self.patterns.clear();
    }

    /// Limpia todos los defaults
    pub fn clear_defaults(&mut self) {
        self.project_defaults.clear();
    }

    /// Limpia toda la memoria
    pub fn clear(&mut self) {
        self.patterns.clear();
        self.project_defaults.clear();
    }

    /// Obtiene el numero de patrones
    pub fn pattern_count(&self) -> usize {
        self.patterns.len()
    }

    /// Obtiene los patrones ordenados por uso (mas usados primero)
    pub fn patterns_by_usage(&self) -> Vec<&Pattern> {
        let mut patterns: Vec<_> = self.patterns.iter().collect();
        patterns.sort_by(|a, b| b.count.cmp(&a.count));
        patterns
    }

    /// Obtiene los patrones ordenados por fecha (mas recientes primero)
    pub fn patterns_by_date(&self) -> Vec<&Pattern> {
        let mut patterns: Vec<_> = self.patterns.iter().collect();
        patterns.sort_by(|a, b| b.last_used.cmp(&a.last_used));
        patterns
    }

    /// Elimina patrones que no se han usado en mas de N dias
    pub fn prune_old_patterns(&mut self, max_age_days: i64) {
        let cutoff = Utc::now() - chrono::Duration::days(max_age_days);
        self.patterns.retain(|p| p.last_used > cutoff);
    }

    /// Elimina un patron especifico por indice
    pub fn remove_pattern(&mut self, index: usize) -> Option<Pattern> {
        if index < self.patterns.len() {
            Some(self.patterns.remove(index))
        } else {
            None
        }
    }
}

impl Pattern {
    /// Crea un nuevo patron
    pub fn new(error: impl Into<String>, context: impl Into<String>, fix: impl Into<String>) -> Self {
        Self {
            error: error.into(),
            context: context.into(),
            fix: fix.into(),
            count: 1,
            last_used: Utc::now(),
        }
    }

    /// Formatea el patron para mostrar en CLI
    pub fn format_display(&self) -> String {
        format!(
            "Error: {}\n  Context: {}\n  Fix: {}\n  Used: {} times (last: {})",
            self.error,
            if self.context.is_empty() { "(none)" } else { &self.context },
            self.fix,
            self.count,
            self.last_used.format("%Y-%m-%d %H:%M")
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_new_memory() {
        let memory = HealingMemory::new();
        assert_eq!(memory.version, MEMORY_VERSION);
        assert!(memory.patterns.is_empty());
        assert!(memory.project_defaults.is_empty());
    }

    #[test]
    fn test_load_nonexistent_file() {
        let result = HealingMemory::load("/nonexistent/path/memory.json");
        assert!(result.is_ok());
        let memory = result.unwrap();
        assert!(memory.patterns.is_empty());
    }

    #[test]
    fn test_save_and_load() {
        let mut memory = HealingMemory::new();
        memory.record_fix("Variable no definida: x", "test context", "x = 1");
        memory.set_default("api_url", "https://example.com");

        // Guardar a archivo temporal
        let file = NamedTempFile::new().unwrap();
        memory.save(file.path()).unwrap();

        // Cargar de nuevo
        let loaded = HealingMemory::load(file.path()).unwrap();
        assert_eq!(loaded.patterns.len(), 1);
        assert_eq!(loaded.patterns[0].error, "Variable no definida: x");
        assert_eq!(loaded.patterns[0].fix, "x = 1");
        assert_eq!(loaded.project_defaults.get("api_url").unwrap(), "https://example.com");
    }

    #[test]
    fn test_find_pattern_exact() {
        let mut memory = HealingMemory::new();
        memory.record_fix("Variable no definida: api_url", "", "api_url = \"https://api.com\"");

        let pattern = memory.find_pattern("Variable no definida: api_url");
        assert!(pattern.is_some());
        assert_eq!(pattern.unwrap().fix, "api_url = \"https://api.com\"");
    }

    #[test]
    fn test_find_pattern_partial() {
        let mut memory = HealingMemory::new();
        memory.record_fix("Variable no definida", "", "define the variable");

        // Buscar con mensaje mas especifico
        let pattern = memory.find_pattern("Variable no definida: foo");
        assert!(pattern.is_some());
    }

    #[test]
    fn test_find_pattern_case_insensitive() {
        let mut memory = HealingMemory::new();
        memory.record_fix("Variable no definida: X", "", "x = 1");

        let pattern = memory.find_pattern("variable no definida: x");
        assert!(pattern.is_some());
    }

    #[test]
    fn test_record_fix_updates_existing() {
        let mut memory = HealingMemory::new();
        memory.record_fix("Error A", "context", "fix 1");
        memory.record_fix("Error A", "context", "fix 2");

        assert_eq!(memory.patterns.len(), 1);
        assert_eq!(memory.patterns[0].count, 2);
        assert_eq!(memory.patterns[0].fix, "fix 2"); // El mas reciente gana
    }

    #[test]
    fn test_project_defaults() {
        let mut memory = HealingMemory::new();
        memory.set_default("api_url", "https://api.example.com");
        memory.set_default("timeout", "30");

        assert_eq!(memory.get_default("api_url").unwrap(), "https://api.example.com");
        assert_eq!(memory.get_default("timeout").unwrap(), "30");
        assert!(memory.get_default("nonexistent").is_none());
    }

    #[test]
    fn test_patterns_by_usage() {
        let mut memory = HealingMemory::new();
        memory.record_fix("Error A", "", "fix A");
        memory.record_fix("Error B", "", "fix B");
        memory.record_fix("Error B", "", "fix B"); // Increment count
        memory.record_fix("Error B", "", "fix B"); // Increment count

        let by_usage = memory.patterns_by_usage();
        assert_eq!(by_usage[0].error, "Error B"); // 3 uses
        assert_eq!(by_usage[1].error, "Error A"); // 1 use
    }

    #[test]
    fn test_clear() {
        let mut memory = HealingMemory::new();
        memory.record_fix("Error", "", "fix");
        memory.set_default("key", "value");

        memory.clear();
        assert!(memory.patterns.is_empty());
        assert!(memory.project_defaults.is_empty());
    }

    #[test]
    fn test_pattern_format_display() {
        let pattern = Pattern::new("Error X", "context Y", "fix Z");
        let display = pattern.format_display();

        assert!(display.contains("Error X"));
        assert!(display.contains("context Y"));
        assert!(display.contains("fix Z"));
    }

    #[test]
    fn test_find_pattern_with_context() {
        let mut memory = HealingMemory::new();
        memory.record_fix("Variable no definida: url", "http.get", "url = \"https://api.com\"");

        // Coincide error y contexto
        let pattern = memory.find_pattern_with_context("Variable no definida: url", "usando http.get");
        assert!(pattern.is_some());

        // Solo coincide error, contexto diferente
        let pattern2 = memory.find_pattern_with_context("Variable no definida: url", "algo completamente diferente");
        assert!(pattern2.is_none());
    }
}
