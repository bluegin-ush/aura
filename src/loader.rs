//! AURA Module Loader
//!
//! Handles file imports with +archivo syntax.
//! When +nombre is encountered and it's not a builtin capability,
//! the loader searches for nombre.aura in the same directory as the main file.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::lexer::{tokenize, Span};
use crate::parser::{parse, Capability, Program, ParseError};

/// Builtin capabilities that should not be treated as file imports
const BUILTIN_CAPABILITIES: &[&str] = &[
    "http", "json", "db", "math", "time", "crypto", "email",
    "auth", "ws", "fs", "env", "core",
];

/// Error during module loading
#[derive(Debug, Clone)]
pub struct LoadError {
    pub message: String,
    pub file: Option<String>,
    pub span: Option<Span>,
}

impl LoadError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            file: None,
            span: None,
        }
    }

    pub fn with_file(mut self, file: impl Into<String>) -> Self {
        self.file = Some(file.into());
        self
    }

    pub fn file_not_found(name: &str, searched_path: &Path) -> Self {
        Self {
            message: format!(
                "No se encontro '{}.aura' (buscado en: {})",
                name,
                searched_path.display()
            ),
            file: None,
            span: None,
        }
    }

    pub fn circular_import(name: &str, chain: &[String]) -> Self {
        Self {
            message: format!(
                "Import circular detectado: {} (cadena: {} -> {})",
                name,
                chain.join(" -> "),
                name
            ),
            file: None,
            span: None,
        }
    }

    pub fn parse_error(file: &str, errors: Vec<ParseError>) -> Self {
        let messages: Vec<String> = errors.iter()
            .map(|e| e.message.clone())
            .collect();
        Self {
            message: format!("Error de parsing en '{}': {}", file, messages.join("; ")),
            file: Some(file.to_string()),
            span: errors.first().map(|e| e.span.clone()),
        }
    }

    pub fn tokenize_error(file: &str, message: &str) -> Self {
        Self {
            message: format!("Error de tokenizacion en '{}': {}", file, message),
            file: Some(file.to_string()),
            span: None,
        }
    }
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(ref file) = self.file {
            write!(f, "[{}] {}", file, self.message)
        } else {
            write!(f, "{}", self.message)
        }
    }
}

/// Check if a capability name is a builtin
pub fn is_builtin_capability(name: &str) -> bool {
    BUILTIN_CAPABILITIES.contains(&name)
}

/// Module loader that resolves imports and combines programs
pub struct Loader {
    /// Base directory for resolving imports
    base_dir: PathBuf,
    /// Set of already imported files (to detect circular imports)
    imported: HashSet<PathBuf>,
    /// Import chain for error messages
    import_chain: Vec<String>,
}

impl Loader {
    /// Create a new loader with the given base directory
    pub fn new(base_dir: impl AsRef<Path>) -> Self {
        Self {
            base_dir: base_dir.as_ref().to_path_buf(),
            imported: HashSet::new(),
            import_chain: Vec::new(),
        }
    }

    /// Create a loader from a file path (uses the file's parent directory)
    pub fn from_file(file_path: impl AsRef<Path>) -> Self {
        let path = file_path.as_ref();
        let base_dir = path.parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."));
        Self::new(base_dir)
    }

    /// Load a program from source code, resolving all imports
    pub fn load_source(&mut self, source: &str, file_name: &str) -> Result<Program, LoadError> {
        // Mark this file as imported
        let file_path = self.base_dir.join(file_name);
        self.imported.insert(file_path.clone());
        self.import_chain.push(file_name.to_string());

        // Tokenize
        let tokens = tokenize(source)
            .map_err(|errors| {
                let msg = errors.iter()
                    .map(|e| e.message.clone())
                    .collect::<Vec<_>>()
                    .join("; ");
                LoadError::tokenize_error(file_name, &msg)
            })?;

        // Parse
        let mut program = parse(tokens)
            .map_err(|errors| LoadError::parse_error(file_name, errors))?;

        // Process imports
        self.resolve_imports(&mut program)?;

        self.import_chain.pop();
        Ok(program)
    }

    /// Load a program from a file path
    pub fn load_file(&mut self, file_path: impl AsRef<Path>) -> Result<Program, LoadError> {
        let path = file_path.as_ref();
        let file_name = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        let source = std::fs::read_to_string(path)
            .map_err(|e| LoadError::new(format!("Error leyendo '{}': {}", path.display(), e)))?;

        self.load_source(&source, file_name)
    }

    /// Resolve imports in a program
    fn resolve_imports(&mut self, program: &mut Program) -> Result<(), LoadError> {
        // Separate builtin capabilities from file imports
        let mut file_imports: Vec<Capability> = Vec::new();
        let mut builtin_caps: Vec<Capability> = Vec::new();

        for cap in program.capabilities.drain(..) {
            if is_builtin_capability(&cap.name) {
                builtin_caps.push(cap);
            } else {
                file_imports.push(cap);
            }
        }

        // Restore builtin capabilities
        program.capabilities = builtin_caps;

        // Process each file import
        for import in file_imports {
            let import_name = &import.name;
            let import_file = format!("{}.aura", import_name);
            let import_path = self.base_dir.join(&import_file);

            // Check for circular imports
            if self.imported.contains(&import_path) {
                return Err(LoadError::circular_import(import_name, &self.import_chain));
            }

            // Check if file exists
            if !import_path.exists() {
                return Err(LoadError::file_not_found(import_name, &import_path));
            }

            // Load the imported file
            let imported_program = self.load_file(&import_path)?;

            // Merge capabilities (avoiding duplicates)
            for cap in imported_program.capabilities {
                if !program.capabilities.iter().any(|c| c.name == cap.name) {
                    program.capabilities.push(cap);
                }
            }

            // Merge definitions
            program.definitions.extend(imported_program.definitions);
        }

        Ok(())
    }
}

/// Convenience function to load a file with all its imports resolved
pub fn load_file(file_path: impl AsRef<Path>) -> Result<Program, LoadError> {
    let path = file_path.as_ref();
    let mut loader = Loader::from_file(path);

    // Read the main file
    let source = std::fs::read_to_string(path)
        .map_err(|e| LoadError::new(format!("Error leyendo '{}': {}", path.display(), e)))?;

    let file_name = path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");

    loader.load_source(&source, file_name)
}

/// Convenience function to load source code with imports resolved
pub fn load_source(source: &str, base_dir: impl AsRef<Path>, file_name: &str) -> Result<Program, LoadError> {
    let mut loader = Loader::new(base_dir);
    loader.load_source(source, file_name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_builtin_capability() {
        assert!(is_builtin_capability("http"));
        assert!(is_builtin_capability("json"));
        assert!(is_builtin_capability("db"));
        assert!(!is_builtin_capability("utils"));
        assert!(!is_builtin_capability("herramientas"));
    }

    #[test]
    fn test_load_simple_source() {
        let source = "+http\nmain = 42\n";
        let mut loader = Loader::new(".");
        let program = loader.load_source(source, "test.aura").unwrap();

        assert_eq!(program.capabilities.len(), 1);
        assert_eq!(program.capabilities[0].name, "http");
        assert_eq!(program.definitions.len(), 1);
    }
}
