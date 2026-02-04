// Type checker básico de AURA
// Verifica que funciones y tipos referenciados existan

use std::collections::HashSet;
use crate::parser::{Program, Definition, Expr, Type, TypeDef, FuncDef};
use crate::lexer::Span;

/// Error de tipo
#[derive(Debug, Clone)]
pub struct TypeError {
    pub message: String,
    pub span: Option<Span>,
    pub suggestion: Option<String>,
}

impl TypeError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            span: None,
            suggestion: None,
        }
    }

    pub fn with_span(mut self, span: Span) -> Self {
        self.span = Some(span);
        self
    }

    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }

    /// Serializa a JSON (para agentes)
    pub fn to_json(&self) -> String {
        serde_json::json!({
            "error": "type_error",
            "message": self.message,
            "span": self.span,
            "suggestion": self.suggestion
        }).to_string()
    }
}

/// Contexto del type checker
#[derive(Debug, Default)]
pub struct TypeContext {
    /// Tipos definidos
    pub types: HashSet<String>,
    /// Funciones definidas
    pub functions: HashSet<String>,
    /// Capacidades habilitadas
    pub capabilities: HashSet<String>,
}

impl TypeContext {
    pub fn new() -> Self {
        let mut ctx = Self::default();
        // Agregar funciones builtin
        ctx.functions.insert("print".to_string());
        ctx.functions.insert("print!".to_string());
        ctx.functions.insert("len".to_string());
        ctx.functions.insert("str".to_string());
        ctx.functions.insert("int".to_string());
        ctx.functions.insert("type".to_string());
        ctx.functions.insert("map".to_string());
        ctx.functions.insert("filter".to_string());
        ctx.functions.insert("first".to_string());
        ctx.functions.insert("last".to_string());
        ctx.functions.insert("sort".to_string());
        ctx.functions.insert("join".to_string());
        ctx
    }

    /// Registra un tipo
    pub fn register_type(&mut self, name: &str) {
        self.types.insert(name.to_string());
    }

    /// Registra una función
    pub fn register_function(&mut self, name: &str) {
        self.functions.insert(name.to_string());
    }

    /// Registra una capacidad
    pub fn register_capability(&mut self, name: &str) {
        self.capabilities.insert(name.to_string());

        // Agregar funciones según la capacidad
        match name {
            "http" => {
                // http.get, http.post, etc. se manejan como métodos
            }
            "json" => {
                self.functions.insert("json".to_string());
            }
            "db" => {
                // Métodos CRUD en tipos
            }
            "fs" => {
                self.functions.insert("read".to_string());
                self.functions.insert("write".to_string());
            }
            "time" => {
                self.functions.insert("now".to_string());
                self.functions.insert("today".to_string());
            }
            _ => {}
        }
    }

    /// Verifica si un tipo existe
    pub fn type_exists(&self, name: &str) -> bool {
        self.types.contains(name)
    }

    /// Verifica si una función existe
    pub fn function_exists(&self, name: &str) -> bool {
        self.functions.contains(name)
    }
}

/// Type checker
pub struct TypeChecker {
    ctx: TypeContext,
    errors: Vec<TypeError>,
}

impl TypeChecker {
    pub fn new() -> Self {
        Self {
            ctx: TypeContext::new(),
            errors: Vec::new(),
        }
    }

    /// Verifica un programa completo
    pub fn check(&mut self, program: &Program) -> Result<(), Vec<TypeError>> {
        // Primera pasada: registrar todos los tipos y funciones
        for cap in &program.capabilities {
            self.ctx.register_capability(&cap.name);
        }

        for def in &program.definitions {
            match def {
                Definition::TypeDef(t) => {
                    self.ctx.register_type(&t.name);
                }
                Definition::FuncDef(f) => {
                    self.ctx.register_function(&f.name);
                }
                _ => {}
            }
        }

        // Segunda pasada: verificar referencias
        for def in &program.definitions {
            match def {
                Definition::TypeDef(t) => {
                    self.check_type_def(t);
                }
                Definition::FuncDef(f) => {
                    self.check_func_def(f);
                }
                _ => {}
            }
        }

        // Verificar que existe main
        if !self.ctx.function_exists("main") {
            self.errors.push(
                TypeError::new("No se encontró función 'main'")
                    .with_suggestion("Agrega: main = <expresión>")
            );
        }

        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(self.errors.clone())
        }
    }

    /// Verifica una definición de tipo
    fn check_type_def(&mut self, ty: &TypeDef) {
        for field in &ty.fields {
            self.check_type(&field.ty);
        }
    }

    /// Verifica un tipo
    fn check_type(&mut self, ty: &Type) {
        match ty {
            Type::Named(name) => {
                if !self.ctx.type_exists(name) && !is_builtin_type(name) {
                    self.errors.push(
                        TypeError::new(format!("Tipo no definido: {}", name))
                            .with_suggestion(format!("Definir: @{} {{ ... }}", name))
                    );
                }
            }
            Type::List(inner) => {
                self.check_type(inner);
            }
            Type::Optional(inner) => {
                self.check_type(inner);
            }
            Type::Map(k, v) => {
                self.check_type(k);
                self.check_type(v);
            }
            // Tipos primitivos siempre válidos
            _ => {}
        }
    }

    /// Verifica una definición de función
    fn check_func_def(&mut self, func: &FuncDef) {
        // Crear contexto local con parámetros
        let mut local_vars: HashSet<String> = HashSet::new();
        for param in &func.params {
            local_vars.insert(param.name.clone());
        }

        self.check_expr(&func.body, &local_vars);
    }

    /// Verifica una expresión
    fn check_expr(&mut self, expr: &Expr, local_vars: &HashSet<String>) {
        match expr {
            Expr::Ident(name) => {
                // Verificar que la variable existe
                if !local_vars.contains(name)
                    && !self.ctx.function_exists(name)
                    && !self.ctx.type_exists(name)
                {
                    self.errors.push(
                        TypeError::new(format!("Identificador no definido: {}", name))
                    );
                }
            }

            Expr::Call { func, args, .. } => {
                // Verificar la función
                if let Expr::Ident(name) = func.as_ref() {
                    if !self.ctx.function_exists(name) && !local_vars.contains(name) {
                        self.errors.push(
                            TypeError::new(format!("Función no definida: {}", name))
                                .with_suggestion(format!("Definir: {}(...) = ...", name))
                        );
                    }
                } else {
                    // Para llamadas como http.get, obj.method, etc.
                    self.check_expr(func, local_vars);
                }

                // Verificar argumentos
                for arg in args {
                    self.check_expr(arg, local_vars);
                }
            }

            Expr::FieldAccess(obj, _field) => {
                self.check_expr(obj, local_vars);
            }

            Expr::SafeAccess(obj, _field) => {
                self.check_expr(obj, local_vars);
            }

            Expr::BinaryOp { left, right, .. } => {
                self.check_expr(left, local_vars);
                self.check_expr(right, local_vars);
            }

            Expr::UnaryOp { expr, .. } => {
                self.check_expr(expr, local_vars);
            }

            Expr::List(items) => {
                for item in items {
                    self.check_expr(item, local_vars);
                }
            }

            Expr::Record(fields) => {
                for (_, value) in fields {
                    self.check_expr(value, local_vars);
                }
            }

            Expr::Pipe(exprs) => {
                for expr in exprs {
                    self.check_expr(expr, local_vars);
                }
            }

            Expr::Lambda { params, body } => {
                let mut new_vars = local_vars.clone();
                for p in params {
                    new_vars.insert(p.clone());
                }
                self.check_expr(body, &new_vars);
            }

            Expr::Let { name, value } => {
                self.check_expr(value, local_vars);
                // El nombre se agrega al scope después
            }

            Expr::If { condition, then_branch, else_branch } => {
                self.check_expr(condition, local_vars);
                self.check_expr(then_branch, local_vars);
                if let Some(else_expr) = else_branch {
                    self.check_expr(else_expr, local_vars);
                }
            }

            Expr::For { var, iter, body } => {
                self.check_expr(iter, local_vars);
                let mut new_vars = local_vars.clone();
                new_vars.insert(var.clone());
                self.check_expr(body, &new_vars);
            }

            Expr::Block(exprs) => {
                for expr in exprs {
                    self.check_expr(expr, local_vars);
                }
            }

            Expr::NullCoalesce(left, right) => {
                self.check_expr(left, local_vars);
                self.check_expr(right, local_vars);
            }

            // Literales y otros no necesitan verificación
            _ => {}
        }
    }
}

impl Default for TypeChecker {
    fn default() -> Self {
        Self::new()
    }
}

/// Verifica si es un tipo builtin
fn is_builtin_type(name: &str) -> bool {
    matches!(name, "int" | "float" | "string" | "bool" | "list" | "record" | "any")
}

/// Función principal de verificación
pub fn check(program: &Program) -> Result<(), Vec<TypeError>> {
    let mut checker = TypeChecker::new();
    checker.check(program)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::tokenize;
    use crate::parser::parse;

    fn check_code(source: &str) -> Result<(), Vec<TypeError>> {
        let tokens = tokenize(source).expect("Tokenize failed");
        let program = parse(tokens).expect("Parse failed");
        check(&program)
    }

    #[test]
    fn test_valid_program() {
        let result = check_code("+http\nmain = 42\n");
        assert!(result.is_ok());
    }

    #[test]
    fn test_missing_main() {
        let result = check_code("+http\nfoo = 42\n");
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.message.contains("main")));
    }

    #[test]
    fn test_undefined_function() {
        let result = check_code("+http\nmain = undefined_func(42)\n");
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.message.contains("undefined_func")));
    }

    #[test]
    fn test_valid_function_call() {
        let result = check_code("+http\ndouble(x) = x * 2\nmain = double(21)\n");
        assert!(result.is_ok());
    }

    #[test]
    fn test_builtin_function() {
        let result = check_code("+http\nmain = len(\"hello\")\n");
        assert!(result.is_ok());
    }
}
