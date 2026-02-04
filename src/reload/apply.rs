//! Aplicacion de cambios al environment de la VM
//!
//! Este modulo toma un `CodeDiff` y lo aplica a una VM existente,
//! agregando nuevas funciones y tipos sin perder el estado actual.

use crate::vm::VM;
use crate::parser::{FuncDef, TypeDef, Program, Definition};
use super::diff::{CodeDiff, ReloadError};

/// Resultado de aplicar un diff
#[derive(Debug, Clone, Default)]
pub struct ApplyResult {
    /// Numero de funciones agregadas
    pub functions_added: usize,
    /// Numero de funciones actualizadas
    pub functions_updated: usize,
    /// Numero de tipos agregados
    pub types_added: usize,
    /// Numero de tipos actualizados
    pub types_updated: usize,
    /// Advertencias durante la aplicacion
    pub warnings: Vec<String>,
}

impl ApplyResult {
    /// Crea un resultado vacio
    pub fn new() -> Self {
        Self::default()
    }

    /// Total de cambios aplicados
    pub fn total_changes(&self) -> usize {
        self.functions_added + self.functions_updated + self.types_added + self.types_updated
    }

    /// Verifica si hubo cambios
    pub fn has_changes(&self) -> bool {
        self.total_changes() > 0
    }

    /// Verifica si hubo advertencias
    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }
}

impl std::fmt::Display for ApplyResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if !self.has_changes() {
            return write!(f, "Sin cambios");
        }

        let mut parts = Vec::new();

        if self.functions_added > 0 {
            parts.push(format!("+{} funciones", self.functions_added));
        }
        if self.functions_updated > 0 {
            parts.push(format!("~{} funciones", self.functions_updated));
        }
        if self.types_added > 0 {
            parts.push(format!("+{} tipos", self.types_added));
        }
        if self.types_updated > 0 {
            parts.push(format!("~{} tipos", self.types_updated));
        }

        write!(f, "{}", parts.join(", "))
    }
}

/// Aplica un diff a la VM
///
/// Esta funcion toma las diferencias calculadas por `compute_diff` y las
/// aplica al environment de la VM, agregando nuevas funciones/tipos y
/// actualizando las existentes.
///
/// # Argumentos
///
/// * `vm` - Referencia mutable a la VM donde aplicar los cambios
/// * `diff` - Las diferencias a aplicar
///
/// # Retorna
///
/// Un `ApplyResult` con estadisticas de los cambios aplicados,
/// o un `ReloadError` si hay problemas.
///
/// # Ejemplo
///
/// ```rust,ignore
/// let diff = compute_diff(&program, "double(x) = x * 2")?;
/// let result = apply_diff(&mut vm, diff)?;
/// println!("Funciones agregadas: {}", result.functions_added);
/// ```
pub fn apply_diff(vm: &mut VM, diff: CodeDiff) -> Result<ApplyResult, ReloadError> {
    let mut result = ApplyResult::new();

    // Aplicar funciones nuevas
    for func in diff.added_functions {
        apply_function(vm, func, &mut result)?;
    }

    // Aplicar funciones modificadas
    for func in diff.modified_functions {
        update_function(vm, func, &mut result)?;
    }

    // Aplicar tipos nuevos
    for ty in diff.added_types {
        apply_type(vm, ty, &mut result)?;
    }

    // Aplicar tipos modificados
    for ty in diff.modified_types {
        update_type(vm, ty, &mut result)?;
    }

    Ok(result)
}

/// Aplica una funcion nueva a la VM
fn apply_function(
    vm: &mut VM,
    func: FuncDef,
    result: &mut ApplyResult,
) -> Result<(), ReloadError> {
    let name = func.name.clone();

    // Verificar que no existe (deberia ser cierto si viene de diff.added_functions)
    if vm.list_functions().contains(&name) {
        result.warnings.push(format!(
            "Funcion '{}' ya existia, se sobrescribira",
            name
        ));
    }

    vm.define_function(func);
    result.functions_added += 1;

    Ok(())
}

/// Actualiza una funcion existente en la VM
fn update_function(
    vm: &mut VM,
    func: FuncDef,
    result: &mut ApplyResult,
) -> Result<(), ReloadError> {
    let name = func.name.clone();

    // La funcion deberia existir (viene de diff.modified_functions)
    if !vm.list_functions().contains(&name) {
        result.warnings.push(format!(
            "Funcion '{}' no existia, se agregara como nueva",
            name
        ));
        result.functions_added += 1;
    } else {
        result.functions_updated += 1;
    }

    // Redefinir la funcion (sobrescribe la anterior)
    vm.define_function(func);

    Ok(())
}

/// Aplica un tipo nuevo a la VM
///
/// Usa el mecanismo de carga de Program para agregar el tipo,
/// ya que VM no expone define_type directamente.
fn apply_type(
    vm: &mut VM,
    ty: TypeDef,
    result: &mut ApplyResult,
) -> Result<(), ReloadError> {
    // Crear un Program temporal con solo este tipo
    let temp_program = Program {
        capabilities: vec![],
        definitions: vec![Definition::TypeDef(ty)],
    };

    // Cargar el programa temporal (agrega el tipo al environment)
    vm.load(&temp_program);
    result.types_added += 1;

    Ok(())
}

/// Actualiza un tipo existente en la VM
fn update_type(
    vm: &mut VM,
    ty: TypeDef,
    result: &mut ApplyResult,
) -> Result<(), ReloadError> {
    let name = ty.name.clone();

    // Advertencia: modificar tipos puede causar problemas con datos existentes
    result.warnings.push(format!(
        "Tipo '{}' modificado - los datos existentes pueden ser incompatibles",
        name
    ));

    // Crear un Program temporal con solo este tipo
    let temp_program = Program {
        capabilities: vec![],
        definitions: vec![Definition::TypeDef(ty)],
    };

    // Cargar el programa temporal (sobrescribe el tipo en el environment)
    vm.load(&temp_program);
    result.types_updated += 1;

    Ok(())
}

/// Aplica codigo directamente a la VM (atajo para compute_diff + apply_diff)
///
/// Esta funcion es un atajo conveniente que combina la deteccion de diff
/// y la aplicacion en un solo paso.
///
/// # Argumentos
///
/// * `vm` - Referencia mutable a la VM
/// * `program` - El programa actual (para comparar)
/// * `new_code` - El nuevo codigo a integrar
///
/// # Retorna
///
/// Un `ApplyResult` con estadisticas, o un error.
pub fn hot_reload(
    vm: &mut VM,
    program: &crate::parser::Program,
    new_code: &str,
) -> Result<ApplyResult, ReloadError> {
    let diff = super::diff::compute_diff(program, new_code)?;
    apply_diff(vm, diff)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::tokenize;
    use crate::parser::parse;

    fn setup_vm(code: &str) -> (VM, crate::parser::Program) {
        let tokens = tokenize(code).unwrap();
        let program = parse(tokens).unwrap();
        let mut vm = VM::new();
        vm.load(&program);
        (vm, program)
    }

    #[test]
    fn test_apply_empty_diff() {
        let (mut vm, _) = setup_vm("+http\nmain = 42\n");

        let diff = CodeDiff::new();
        let result = apply_diff(&mut vm, diff).unwrap();

        assert_eq!(result.total_changes(), 0);
        assert!(!result.has_changes());
    }

    #[test]
    fn test_apply_new_function() {
        let (mut vm, program) = setup_vm("+http\nmain = 42\n");

        let diff = super::super::diff::compute_diff(&program, "double(x) = x * 2").unwrap();
        let result = apply_diff(&mut vm, diff).unwrap();

        assert_eq!(result.functions_added, 1);
        assert_eq!(result.functions_updated, 0);
        assert!(vm.list_functions().contains(&"double".to_string()));
    }

    #[test]
    fn test_apply_modified_function() {
        let (mut vm, program) = setup_vm("+http\ndouble(x) = x * 2\n");

        let diff = super::super::diff::compute_diff(&program, "double(x) = x + x").unwrap();
        let result = apply_diff(&mut vm, diff).unwrap();

        assert_eq!(result.functions_added, 0);
        assert_eq!(result.functions_updated, 1);
    }

    #[test]
    fn test_apply_new_type() {
        let (mut vm, program) = setup_vm("+http\nmain = 42\n");

        let diff = super::super::diff::compute_diff(
            &program,
            "@User {\n  id:i @pk\n  name:s\n}",
        )
        .unwrap();

        let result = apply_diff(&mut vm, diff).unwrap();

        assert_eq!(result.types_added, 1);
    }

    #[test]
    fn test_apply_modified_type_warning() {
        let (mut vm, program) = setup_vm("+http\n@User {\n  id:i @pk\n  name:s\n}\n");

        let diff = super::super::diff::compute_diff(
            &program,
            "@User {\n  id:i @pk\n  name:s\n  email:s?\n}",
        )
        .unwrap();

        let result = apply_diff(&mut vm, diff).unwrap();

        assert_eq!(result.types_updated, 1);
        assert!(result.has_warnings());
        assert!(result.warnings[0].contains("User"));
    }

    #[test]
    fn test_apply_result_display() {
        let mut result = ApplyResult::new();
        result.functions_added = 2;
        result.functions_updated = 1;
        result.types_added = 1;

        let display = result.to_string();
        assert!(display.contains("+2 funciones"));
        assert!(display.contains("~1 funciones"));
        assert!(display.contains("+1 tipos"));
    }

    #[test]
    fn test_hot_reload_shortcut() {
        let (mut vm, program) = setup_vm("+http\nmain = 42\n");

        let result = hot_reload(&mut vm, &program, "double(x) = x * 2").unwrap();

        assert_eq!(result.functions_added, 1);
        assert!(vm.list_functions().contains(&"double".to_string()));
    }

    #[test]
    fn test_multiple_apply_operations() {
        let (mut vm, program) = setup_vm("+http\nmain = 42\n");

        // Primera aplicacion
        let result1 = hot_reload(&mut vm, &program, "double(x) = x * 2").unwrap();
        assert_eq!(result1.functions_added, 1);

        // Segunda aplicacion (agregar otra funcion)
        // Nota: El programa original no incluye double, asi que double seria "nuevo" otra vez
        // En un caso real, deberiamos actualizar el programa despues de cada reload
        let result2 = hot_reload(&mut vm, &program, "triple(x) = x * 3").unwrap();
        assert_eq!(result2.functions_added, 1);

        let funcs = vm.list_functions();
        assert!(funcs.contains(&"double".to_string()));
        assert!(funcs.contains(&"triple".to_string()));
    }
}
