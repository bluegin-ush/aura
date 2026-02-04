//! Deteccion de diferencias entre codigo viejo y nuevo
//!
//! Este modulo compara el estado actual del programa con nuevo codigo
//! para determinar que funciones y tipos fueron agregados o modificados.

use std::collections::HashMap;
use crate::lexer::tokenize;
use crate::parser::{parse, Program, Definition, FuncDef, TypeDef};

/// Error durante el proceso de hot reload
#[derive(Debug, Clone)]
pub enum ReloadError {
    /// Error al tokenizar el nuevo codigo
    LexError(String),
    /// Error al parsear el nuevo codigo
    ParseError(String),
    /// Error de compatibilidad (ej: cambiar tipo de una variable en uso)
    IncompatibleChange(String),
    /// Error al aplicar cambios
    ApplyError(String),
}

impl std::fmt::Display for ReloadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReloadError::LexError(msg) => write!(f, "Error de tokenizacion: {}", msg),
            ReloadError::ParseError(msg) => write!(f, "Error de parseo: {}", msg),
            ReloadError::IncompatibleChange(msg) => write!(f, "Cambio incompatible: {}", msg),
            ReloadError::ApplyError(msg) => write!(f, "Error al aplicar: {}", msg),
        }
    }
}

impl std::error::Error for ReloadError {}

/// Diferencias detectadas entre el programa actual y el nuevo codigo
#[derive(Debug, Clone, Default)]
pub struct CodeDiff {
    /// Funciones completamente nuevas
    pub added_functions: Vec<FuncDef>,
    /// Funciones que ya existian pero fueron modificadas
    pub modified_functions: Vec<FuncDef>,
    /// Tipos completamente nuevos
    pub added_types: Vec<TypeDef>,
    /// Tipos que ya existian pero fueron modificados
    pub modified_types: Vec<TypeDef>,
}

impl CodeDiff {
    /// Crea un CodeDiff vacio
    pub fn new() -> Self {
        Self::default()
    }

    /// Verifica si hay cambios
    pub fn is_empty(&self) -> bool {
        self.added_functions.is_empty()
            && self.modified_functions.is_empty()
            && self.added_types.is_empty()
            && self.modified_types.is_empty()
    }

    /// Numero total de cambios
    pub fn total_changes(&self) -> usize {
        self.added_functions.len()
            + self.modified_functions.len()
            + self.added_types.len()
            + self.modified_types.len()
    }
}

/// Compara el programa actual con nuevo codigo y detecta diferencias
///
/// # Argumentos
///
/// * `old_program` - El programa actualmente cargado en la VM
/// * `new_code` - Codigo fuente nuevo a integrar
///
/// # Retorna
///
/// Un `CodeDiff` con las funciones y tipos nuevos/modificados,
/// o un `ReloadError` si hay problemas de sintaxis o incompatibilidad.
///
/// # Ejemplo
///
/// ```rust,ignore
/// let diff = compute_diff(&program, "double(x) = x * 2")?;
/// ```
pub fn compute_diff(old_program: &Program, new_code: &str) -> Result<CodeDiff, ReloadError> {
    // Tokenizar nuevo codigo
    let tokens = tokenize(new_code).map_err(|errors| {
        let messages: Vec<String> = errors.iter().map(|e| e.message.clone()).collect();
        ReloadError::LexError(messages.join("; "))
    })?;

    // Parsear nuevo codigo
    let new_program = parse(tokens).map_err(|errors| {
        let messages: Vec<String> = errors.iter().map(|e| e.message.clone()).collect();
        ReloadError::ParseError(messages.join("; "))
    })?;

    // Construir indices del programa viejo
    let old_functions = build_function_index(old_program);
    let old_types = build_type_index(old_program);

    let mut diff = CodeDiff::new();

    // Comparar definiciones del nuevo codigo
    for def in &new_program.definitions {
        match def {
            Definition::FuncDef(func) => {
                if let Some(old_func) = old_functions.get(&func.name) {
                    // La funcion ya existe - verificar si cambio
                    if !functions_equal(old_func, func) {
                        diff.modified_functions.push(func.clone());
                    }
                } else {
                    // Funcion nueva
                    diff.added_functions.push(func.clone());
                }
            }
            Definition::TypeDef(ty) => {
                if let Some(old_type) = old_types.get(&ty.name) {
                    // El tipo ya existe - verificar si cambio
                    if !types_equal(old_type, ty) {
                        diff.modified_types.push(ty.clone());
                    }
                } else {
                    // Tipo nuevo
                    diff.added_types.push(ty.clone());
                }
            }
            // Ignorar otras definiciones por ahora (EnumDef, ApiDef, TestDef)
            _ => {}
        }
    }

    Ok(diff)
}

/// Construye un indice de funciones por nombre
fn build_function_index(program: &Program) -> HashMap<String, &FuncDef> {
    let mut index = HashMap::new();
    for def in &program.definitions {
        if let Definition::FuncDef(func) = def {
            index.insert(func.name.clone(), func);
        }
    }
    index
}

/// Construye un indice de tipos por nombre
fn build_type_index(program: &Program) -> HashMap<String, &TypeDef> {
    let mut index = HashMap::new();
    for def in &program.definitions {
        if let Definition::TypeDef(ty) = def {
            index.insert(ty.name.clone(), ty);
        }
    }
    index
}

/// Compara dos funciones para ver si son iguales
///
/// Ignora el span ya que puede cambiar entre versiones.
fn functions_equal(a: &FuncDef, b: &FuncDef) -> bool {
    // Comparar nombre
    if a.name != b.name {
        return false;
    }

    // Comparar efecto
    if a.has_effect != b.has_effect {
        return false;
    }

    // Comparar parametros (nombre y tipo)
    if a.params.len() != b.params.len() {
        return false;
    }
    for (pa, pb) in a.params.iter().zip(b.params.iter()) {
        if pa.name != pb.name || pa.ty != pb.ty {
            return false;
        }
    }

    // Comparar tipo de retorno
    if a.return_type != b.return_type {
        return false;
    }

    // Comparar cuerpo
    // Nota: Esta comparacion es estructural, no semantica
    a.body == b.body
}

/// Compara dos tipos para ver si son iguales
///
/// Ignora el span ya que puede cambiar entre versiones.
fn types_equal(a: &TypeDef, b: &TypeDef) -> bool {
    // Comparar nombre
    if a.name != b.name {
        return false;
    }

    // Comparar campos
    if a.fields.len() != b.fields.len() {
        return false;
    }
    for (fa, fb) in a.fields.iter().zip(b.fields.iter()) {
        if fa.name != fb.name || fa.ty != fb.ty || fa.nullable != fb.nullable {
            return false;
        }
        // Comparar anotaciones (ignorando span)
        if fa.annotations.len() != fb.annotations.len() {
            return false;
        }
        for (aa, ab) in fa.annotations.iter().zip(fb.annotations.iter()) {
            if aa.name != ab.name || aa.args != ab.args {
                return false;
            }
        }
    }

    // Comparar anotaciones del tipo
    if a.annotations.len() != b.annotations.len() {
        return false;
    }
    for (aa, ab) in a.annotations.iter().zip(b.annotations.iter()) {
        if aa.name != ab.name || aa.args != ab.args {
            return false;
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_program(code: &str) -> Program {
        let tokens = tokenize(code).unwrap();
        parse(tokens).unwrap()
    }

    #[test]
    fn test_detect_added_function() {
        let old = parse_program("+http\nmain = 42\n");
        let diff = compute_diff(&old, "double(x) = x * 2").unwrap();

        assert_eq!(diff.added_functions.len(), 1);
        assert_eq!(diff.added_functions[0].name, "double");
        assert!(diff.modified_functions.is_empty());
    }

    #[test]
    fn test_detect_modified_function() {
        let old = parse_program("+http\ndouble(x) = x * 2\n");
        let diff = compute_diff(&old, "double(x) = x + x").unwrap();

        assert!(diff.added_functions.is_empty());
        assert_eq!(diff.modified_functions.len(), 1);
        assert_eq!(diff.modified_functions[0].name, "double");
    }

    #[test]
    fn test_detect_unchanged_function() {
        let old = parse_program("+http\ndouble(x) = x * 2\n");
        let diff = compute_diff(&old, "double(x) = x * 2").unwrap();

        assert!(diff.is_empty());
    }

    #[test]
    fn test_detect_added_type() {
        let old = parse_program("+http\nmain = 42\n");
        let diff = compute_diff(&old, "@User {\n  id:i @pk\n  name:s\n}").unwrap();

        assert_eq!(diff.added_types.len(), 1);
        assert_eq!(diff.added_types[0].name, "User");
    }

    #[test]
    fn test_detect_modified_type() {
        let old = parse_program("+http\n@User {\n  id:i @pk\n  name:s\n}\n");
        let diff = compute_diff(&old, "@User {\n  id:i @pk\n  name:s\n  email:s?\n}").unwrap();

        assert!(diff.added_types.is_empty());
        assert_eq!(diff.modified_types.len(), 1);
        assert_eq!(diff.modified_types[0].name, "User");
    }

    #[test]
    fn test_multiple_changes() {
        let old = parse_program("+http\nmain = 42\ndouble(x) = x * 2\n");
        let new_code = r#"
double(x) = x + x
triple(x) = x * 3
@User {
  id:i @pk
}
"#;
        let diff = compute_diff(&old, new_code).unwrap();

        assert_eq!(diff.added_functions.len(), 1); // triple
        assert_eq!(diff.modified_functions.len(), 1); // double
        assert_eq!(diff.added_types.len(), 1); // User
    }

    #[test]
    fn test_lex_error() {
        let old = parse_program("+http\nmain = 42\n");
        // Caracter invalido
        let result = compute_diff(&old, "func = `invalid`");
        assert!(matches!(result, Err(ReloadError::LexError(_))));
    }

    #[test]
    fn test_parse_error() {
        let old = parse_program("+http\nmain = 42\n");
        // Parentesis no cerrado
        let result = compute_diff(&old, "func(x = x * 2");
        assert!(matches!(result, Err(ReloadError::ParseError(_))));
    }

    #[test]
    fn test_function_param_change_is_modification() {
        let old = parse_program("+http\nadd(a, b) = a + b\n");
        // Cambiar nombre de parametro
        let diff = compute_diff(&old, "add(x, y) = x + y").unwrap();

        assert_eq!(diff.modified_functions.len(), 1);
    }

    #[test]
    fn test_effect_marker_change() {
        let old = parse_program("+http\nfetch(url) = url\n");
        // Agregar marcador de efecto
        let diff = compute_diff(&old, "fetch!(url) = url").unwrap();

        assert_eq!(diff.modified_functions.len(), 1);
    }
}
