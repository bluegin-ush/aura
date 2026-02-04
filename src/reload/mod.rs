//! Hot Reload Module for AURA
//!
//! Permite agregar codigo sin reiniciar el runtime.
//!
//! ## Uso
//!
//! ```rust,ignore
//! use aura::reload::{compute_diff, apply_diff};
//! use aura::vm::VM;
//!
//! let mut vm = VM::new();
//! // ... cargar programa inicial ...
//!
//! // El usuario envia nuevo codigo
//! let new_code = "double(x) = x * 2";
//!
//! // Calcular diferencias
//! let diff = compute_diff(&old_program, new_code)?;
//!
//! // Aplicar cambios sin perder estado
//! let result = apply_diff(&mut vm, diff)?;
//! println!("Funciones agregadas: {}", result.functions_added);
//! ```

pub mod apply;
pub mod diff;

pub use apply::{apply_diff, hot_reload, ApplyResult};
pub use diff::{compute_diff, CodeDiff, ReloadError};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::tokenize;
    use crate::parser::parse;
    use crate::vm::VM;

    #[test]
    fn test_hot_reload_add_function() {
        // Programa inicial
        let initial_code = "+http\nmain = 42\n";
        let tokens = tokenize(initial_code).unwrap();
        let program = parse(tokens).unwrap();

        let mut vm = VM::new();
        vm.load(&program);

        // Nuevo codigo con funcion adicional
        let new_code = "double(x) = x * 2";

        // Calcular diff
        let diff = compute_diff(&program, new_code).unwrap();

        assert_eq!(diff.added_functions.len(), 1);
        assert_eq!(diff.added_functions[0].name, "double");

        // Aplicar diff
        let result = apply_diff(&mut vm, diff).unwrap();

        assert_eq!(result.functions_added, 1);
        assert_eq!(result.functions_updated, 0);

        // Verificar que la funcion fue agregada
        assert!(vm.list_functions().contains(&"double".to_string()));
    }

    #[test]
    fn test_hot_reload_modify_function() {
        // Programa inicial con una funcion
        let initial_code = "+http\ndouble(x) = x * 2\nmain = double(5)\n";
        let tokens = tokenize(initial_code).unwrap();
        let program = parse(tokens).unwrap();

        let mut vm = VM::new();
        vm.load(&program);

        // Nueva version de la funcion
        let new_code = "double(x) = x + x";

        // Calcular diff
        let diff = compute_diff(&program, new_code).unwrap();

        assert_eq!(diff.modified_functions.len(), 1);
        assert_eq!(diff.modified_functions[0].name, "double");

        // Aplicar diff
        let result = apply_diff(&mut vm, diff).unwrap();

        assert_eq!(result.functions_added, 0);
        assert_eq!(result.functions_updated, 1);
    }

    #[test]
    fn test_hot_reload_add_type() {
        // Programa inicial
        let initial_code = "+http\nmain = 42\n";
        let tokens = tokenize(initial_code).unwrap();
        let program = parse(tokens).unwrap();

        let mut vm = VM::new();
        vm.load(&program);

        // Nuevo tipo
        let new_code = "@User {\n  id:i @pk\n  name:s\n}";

        // Calcular diff
        let diff = compute_diff(&program, new_code).unwrap();

        assert_eq!(diff.added_types.len(), 1);
        assert_eq!(diff.added_types[0].name, "User");

        // Aplicar diff
        let result = apply_diff(&mut vm, diff).unwrap();

        assert_eq!(result.types_added, 1);
    }

    #[test]
    fn test_hot_reload_multiple_additions() {
        // Programa inicial
        let initial_code = "+http\nmain = 42\n";
        let tokens = tokenize(initial_code).unwrap();
        let program = parse(tokens).unwrap();

        let mut vm = VM::new();
        vm.load(&program);

        // Multiples nuevas definiciones
        let new_code = r#"@User {
  id:i @pk
  name:s
}

double(x) = x * 2
triple(x) = x * 3
"#;

        // Calcular diff
        let diff = compute_diff(&program, new_code).unwrap();

        assert_eq!(diff.added_types.len(), 1);
        assert_eq!(diff.added_functions.len(), 2);

        // Aplicar diff
        let result = apply_diff(&mut vm, diff).unwrap();

        assert_eq!(result.types_added, 1);
        assert_eq!(result.functions_added, 2);

        // Verificar
        let funcs = vm.list_functions();
        assert!(funcs.contains(&"double".to_string()));
        assert!(funcs.contains(&"triple".to_string()));
    }

    #[test]
    fn test_hot_reload_parse_error() {
        let initial_code = "+http\nmain = 42\n";
        let tokens = tokenize(initial_code).unwrap();
        let program = parse(tokens).unwrap();

        // Codigo con error de sintaxis
        let new_code = "double(x = x * 2"; // falta )

        let result = compute_diff(&program, new_code);
        assert!(result.is_err());

        if let Err(ReloadError::ParseError(_)) = result {
            // OK
        } else {
            panic!("Expected ParseError");
        }
    }
}
