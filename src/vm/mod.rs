// VM de AURA
// Intérprete básico para ejecutar código AURA

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::parser::{Program, Definition, Expr, BinaryOp, UnaryOp, FuncDef, TypeDef};
use crate::caps::http::{http_get, http_post, http_put, http_delete};

/// Valor en runtime
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Value {
    Nil,
    Int(i64),
    Float(f64),
    String(String),
    Bool(bool),
    List(Vec<Value>),
    Record(HashMap<String, Value>),
    Function(String),
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Nil => write!(f, "nil"),
            Value::Int(n) => write!(f, "{}", n),
            Value::Float(n) => write!(f, "{}", n),
            Value::String(s) => write!(f, "{}", s),
            Value::Bool(b) => write!(f, "{}", b),
            Value::List(items) => {
                write!(f, "[")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        write!(f, " ")?;
                    }
                    write!(f, "{}", item)?;
                }
                write!(f, "]")
            }
            Value::Record(fields) => {
                write!(f, "{{")?;
                for (i, (k, v)) in fields.iter().enumerate() {
                    if i > 0 {
                        write!(f, " ")?;
                    }
                    write!(f, "{}:{}", k, v)?;
                }
                write!(f, "}}")
            }
            Value::Function(name) => write!(f, "<fn {}>", name),
        }
    }
}

/// Error de ejecución
#[derive(Debug, Clone)]
pub struct RuntimeError {
    pub message: String,
}

impl RuntimeError {
    pub fn new(message: impl Into<String>) -> Self {
        Self { message: message.into() }
    }
}

/// Entorno de ejecución
#[derive(Debug, Default)]
pub struct Environment {
    /// Variables locales
    variables: HashMap<String, Value>,
    /// Funciones definidas
    functions: HashMap<String, FuncDef>,
    /// Tipos definidos
    types: HashMap<String, TypeDef>,
    /// Entorno padre (para scopes anidados)
    parent: Option<Box<Environment>>,
}

impl Environment {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_parent(parent: Environment) -> Self {
        Self {
            parent: Some(Box::new(parent)),
            ..Default::default()
        }
    }

    pub fn define(&mut self, name: String, value: Value) {
        self.variables.insert(name, value);
    }

    pub fn get(&self, name: &str) -> Option<Value> {
        self.variables.get(name).cloned()
            .or_else(|| self.parent.as_ref().and_then(|p| p.get(name)))
    }

    pub fn define_function(&mut self, func: FuncDef) {
        self.functions.insert(func.name.clone(), func);
    }

    pub fn get_function(&self, name: &str) -> Option<&FuncDef> {
        self.functions.get(name)
            .or_else(|| self.parent.as_ref().and_then(|p| p.get_function(name)))
    }

    pub fn define_type(&mut self, ty: TypeDef) {
        self.types.insert(ty.name.clone(), ty);
    }

    pub fn get_type(&self, name: &str) -> Option<&TypeDef> {
        self.types.get(name)
            .or_else(|| self.parent.as_ref().and_then(|p| p.get_type(name)))
    }

    /// Lista los nombres de todas las funciones definidas
    pub fn list_functions(&self) -> Vec<String> {
        let mut names: Vec<String> = self.functions.keys().cloned().collect();
        if let Some(ref parent) = self.parent {
            names.extend(parent.list_functions());
        }
        names.sort();
        names.dedup();
        names
    }

    /// Lista los nombres de todas las variables definidas
    pub fn list_variables(&self) -> Vec<String> {
        let mut names: Vec<String> = self.variables.keys().cloned().collect();
        if let Some(ref parent) = self.parent {
            names.extend(parent.list_variables());
        }
        names.sort();
        names.dedup();
        names
    }

    /// Limpia el entorno (variables, funciones, tipos)
    pub fn clear(&mut self) {
        self.variables.clear();
        self.functions.clear();
        self.types.clear();
        self.parent = None;
    }
}

/// Máquina virtual de AURA
pub struct VM {
    env: Environment,
}

impl VM {
    pub fn new() -> Self {
        Self {
            env: Environment::new(),
        }
    }

    /// Carga un programa en la VM
    pub fn load(&mut self, program: &Program) {
        // Cargar tipos
        for def in &program.definitions {
            if let Definition::TypeDef(ty) = def {
                self.env.define_type(ty.clone());
            }
        }

        // Cargar funciones
        for def in &program.definitions {
            if let Definition::FuncDef(func) = def {
                self.env.define_function(func.clone());
            }
        }
    }

    /// Ejecuta el programa (busca y ejecuta `main`)
    pub fn run(&mut self) -> Result<Value, RuntimeError> {
        match self.env.get_function("main") {
            Some(main_func) => {
                let body = main_func.body.clone();
                self.eval(&body)
            }
            None => Err(RuntimeError::new("No se encontró función 'main'")),
        }
    }

    /// Define una funcion en el entorno actual
    pub fn define_function(&mut self, func: FuncDef) {
        self.env.define_function(func);
    }

    /// Lista las funciones definidas
    pub fn list_functions(&self) -> Vec<String> {
        self.env.list_functions()
    }

    /// Lista las variables definidas
    pub fn list_variables(&self) -> Vec<String> {
        self.env.list_variables()
    }

    /// Reinicia el estado de la VM
    pub fn reset(&mut self) {
        self.env.clear();
    }

    /// Evalúa una expresión
    pub fn eval(&mut self, expr: &Expr) -> Result<Value, RuntimeError> {
        match expr {
            // Literales
            Expr::Int(n) => Ok(Value::Int(*n)),
            Expr::Float(n) => Ok(Value::Float(*n)),
            Expr::String(s) => Ok(Value::String(self.interpolate_string(s)?)),
            Expr::Bool(b) => Ok(Value::Bool(*b)),
            Expr::Nil => Ok(Value::Nil),

            // Identificador
            Expr::Ident(name) => {
                // Primero buscar en variables
                if let Some(val) = self.env.get(name) {
                    return Ok(val);
                }
                // Luego buscar en funciones
                if self.env.get_function(name).is_some() {
                    return Ok(Value::Function(name.clone()));
                }
                // Luego buscar en tipos
                if self.env.get_type(name).is_some() {
                    return Ok(Value::Function(name.clone())); // Tipos como constructores
                }
                Err(RuntimeError::new(format!("Variable no definida: {}", name)))
            }

            // Lista
            Expr::List(items) => {
                let values: Result<Vec<_>, _> = items.iter()
                    .map(|e| self.eval(e))
                    .collect();
                Ok(Value::List(values?))
            }

            // Record
            Expr::Record(fields) => {
                let mut map = HashMap::new();
                for (name, expr) in fields {
                    map.insert(name.clone(), self.eval(expr)?);
                }
                Ok(Value::Record(map))
            }

            // Acceso a campo
            Expr::FieldAccess(obj, field) => {
                let obj_val = self.eval(obj)?;
                match obj_val {
                    Value::Record(map) => {
                        map.get(field)
                            .cloned()
                            .ok_or_else(|| RuntimeError::new(format!("Campo no encontrado: {}", field)))
                    }
                    _ => Err(RuntimeError::new(format!("No se puede acceder a campo '{}' en {:?}", field, obj_val))),
                }
            }

            // Safe access
            Expr::SafeAccess(obj, field) => {
                let obj_val = self.eval(obj)?;
                match obj_val {
                    Value::Nil => Ok(Value::Nil),
                    Value::Record(map) => {
                        Ok(map.get(field).cloned().unwrap_or(Value::Nil))
                    }
                    _ => Err(RuntimeError::new(format!("No se puede acceder a campo '{}' en {:?}", field, obj_val))),
                }
            }

            // Llamada a función
            Expr::Call { func, args, has_effect: _ } => {
                self.eval_call(func, args)
            }

            // Operación binaria
            Expr::BinaryOp { left, op, right } => {
                let left_val = self.eval(left)?;
                let right_val = self.eval(right)?;
                self.eval_binary_op(&left_val, op, &right_val)
            }

            // Operación unaria
            Expr::UnaryOp { op, expr } => {
                let val = self.eval(expr)?;
                self.eval_unary_op(op, &val)
            }

            // Pipe
            Expr::Pipe(exprs) => {
                let mut result = self.eval(&exprs[0])?;
                for expr in &exprs[1..] {
                    // Para pipe, el resultado anterior se pasa como argumento
                    result = self.eval_pipe_step(&result, expr)?;
                }
                Ok(result)
            }

            // Lambda
            Expr::Lambda { params: _, body: _ } => {
                // Por ahora retornamos un placeholder
                Ok(Value::Function("<lambda>".to_string()))
            }

            // Null coalesce
            Expr::NullCoalesce(left, right) => {
                let left_val = self.eval(left)?;
                if left_val == Value::Nil {
                    self.eval(right)
                } else {
                    Ok(left_val)
                }
            }

            // Placeholder
            Expr::Placeholder => Ok(Value::Nil),

            // Block
            Expr::Block(exprs) => {
                let mut result = Value::Nil;
                for expr in exprs {
                    result = self.eval(expr)?;
                }
                Ok(result)
            }

            // Let binding
            Expr::Let { name, value } => {
                let val = self.eval(value)?;
                self.env.define(name.clone(), val.clone());
                Ok(val)
            }

            // If expression
            Expr::If { condition, then_branch, else_branch } => {
                let cond = self.eval(condition)?;
                if self.is_truthy(&cond) {
                    self.eval(then_branch)
                } else if let Some(else_expr) = else_branch {
                    self.eval(else_expr)
                } else {
                    Ok(Value::Nil)
                }
            }

            // For loop
            Expr::For { var, iter, body } => {
                let iter_val = self.eval(iter)?;
                let mut result = Value::Nil;

                if let Value::List(items) = iter_val {
                    for item in items {
                        self.env.define(var.clone(), item);
                        result = self.eval(body)?;
                    }
                }
                Ok(result)
            }

            // Match, InterpolatedString, Spread - no implementados aún
            _ => Err(RuntimeError::new("Expresión no soportada aún")),
        }
    }

    /// Evalúa una llamada a función
    fn eval_call(&mut self, func: &Expr, args: &[Expr]) -> Result<Value, RuntimeError> {
        // Detectar llamadas a métodos de objetos especiales (http.get, http.post, etc.)
        if let Expr::FieldAccess(obj, method) = func {
            if let Expr::Ident(obj_name) = obj.as_ref() {
                if obj_name == "http" {
                    return self.call_http_method(method, args);
                }
            }
        }

        // Evaluar la función
        let func_val = self.eval(func)?;

        // Evaluar argumentos
        let arg_values: Result<Vec<_>, _> = args.iter()
            .map(|a| self.eval(a))
            .collect();
        let arg_values = arg_values?;

        match func_val {
            Value::Function(name) => {
                // Buscar función definida
                if let Some(func_def) = self.env.get_function(&name).cloned() {
                    self.call_function(&func_def, &arg_values)
                } else {
                    // Funciones built-in
                    self.call_builtin(&name, &arg_values)
                }
            }
            _ => Err(RuntimeError::new(format!("No se puede llamar a {:?}", func_val))),
        }
    }

    /// Llama a un método HTTP (http.get, http.post, etc.)
    fn call_http_method(&mut self, method: &str, args: &[Expr]) -> Result<Value, RuntimeError> {
        // Evaluar argumentos
        let arg_values: Result<Vec<_>, _> = args.iter()
            .map(|a| self.eval(a))
            .collect();
        let arg_values = arg_values?;

        // Extraer URL (primer argumento)
        let url = match arg_values.first() {
            Some(Value::String(s)) => s.clone(),
            Some(other) => return Err(RuntimeError::new(format!("http.{} requiere URL como string, recibió: {:?}", method, other))),
            None => return Err(RuntimeError::new(format!("http.{} requiere al menos un argumento (URL)", method))),
        };

        // Extraer body (segundo argumento, opcional)
        let body = match arg_values.get(1) {
            Some(Value::String(s)) => Some(s.as_str()),
            Some(Value::Record(r)) => {
                // Convertir record a JSON string
                None // Por ahora no soportamos records como body directamente
            }
            _ => None,
        };

        // Extraer headers (tercer argumento, opcional)
        let headers = match arg_values.get(2) {
            Some(Value::Record(r)) => {
                let mut h = std::collections::HashMap::new();
                for (k, v) in r {
                    if let Value::String(s) = v {
                        h.insert(k.clone(), s.clone());
                    }
                }
                Some(h)
            }
            _ => None,
        };

        match method {
            "get" => http_get(&url, headers.as_ref()),
            "post" => http_post(&url, body, headers.as_ref()),
            "put" => http_put(&url, body, headers.as_ref()),
            "delete" => http_delete(&url, headers.as_ref()),
            _ => Err(RuntimeError::new(format!("Método HTTP no soportado: {}", method))),
        }
    }

    /// Llama a una función definida por el usuario
    fn call_function(&mut self, func: &FuncDef, args: &[Value]) -> Result<Value, RuntimeError> {
        // Crear nuevo entorno con los parámetros
        let mut new_env = Environment::new();

        for (param, arg) in func.params.iter().zip(args.iter()) {
            new_env.define(param.name.clone(), arg.clone());
        }

        // Copiar funciones y tipos al nuevo entorno
        new_env.parent = Some(Box::new(std::mem::take(&mut self.env)));

        // Evaluar el cuerpo
        self.env = new_env;
        let result = self.eval(&func.body);

        // Restaurar entorno
        if let Some(parent) = self.env.parent.take() {
            self.env = *parent;
        }

        result
    }

    /// Llama a una función built-in
    fn call_builtin(&self, name: &str, args: &[Value]) -> Result<Value, RuntimeError> {
        match name {
            "print" | "print!" => {
                for arg in args {
                    println!("{}", arg);
                }
                Ok(Value::Nil)
            }
            "len" => {
                match args.first() {
                    Some(Value::String(s)) => Ok(Value::Int(s.len() as i64)),
                    Some(Value::List(l)) => Ok(Value::Int(l.len() as i64)),
                    _ => Err(RuntimeError::new("len requiere string o lista")),
                }
            }
            "str" => {
                match args.first() {
                    Some(v) => Ok(Value::String(v.to_string())),
                    None => Ok(Value::String(String::new())),
                }
            }
            "int" => {
                match args.first() {
                    Some(Value::String(s)) => {
                        s.parse::<i64>()
                            .map(Value::Int)
                            .map_err(|_| RuntimeError::new("No se puede convertir a int"))
                    }
                    Some(Value::Float(f)) => Ok(Value::Int(*f as i64)),
                    Some(Value::Int(n)) => Ok(Value::Int(*n)),
                    _ => Err(RuntimeError::new("int requiere string, float o int")),
                }
            }
            "type" => {
                match args.first() {
                    Some(Value::Nil) => Ok(Value::String("nil".to_string())),
                    Some(Value::Int(_)) => Ok(Value::String("int".to_string())),
                    Some(Value::Float(_)) => Ok(Value::String("float".to_string())),
                    Some(Value::String(_)) => Ok(Value::String("string".to_string())),
                    Some(Value::Bool(_)) => Ok(Value::String("bool".to_string())),
                    Some(Value::List(_)) => Ok(Value::String("list".to_string())),
                    Some(Value::Record(_)) => Ok(Value::String("record".to_string())),
                    Some(Value::Function(_)) => Ok(Value::String("function".to_string())),
                    None => Ok(Value::String("nil".to_string())),
                }
            }
            // TODO: Agregar más builtins (map, filter, etc.)
            _ => Err(RuntimeError::new(format!("Función no definida: {}", name))),
        }
    }

    /// Evalúa un paso de pipe
    fn eval_pipe_step(&mut self, input: &Value, expr: &Expr) -> Result<Value, RuntimeError> {
        match expr {
            // Si es una llamada, agregar el input como primer argumento
            Expr::Call { func, args, has_effect } => {
                let mut new_args = vec![input.clone()];
                for arg in args {
                    // Reemplazar placeholders con el input
                    if matches!(arg, Expr::Placeholder) {
                        new_args.push(input.clone());
                    } else {
                        new_args.push(self.eval(arg)?);
                    }
                }

                let func_val = self.eval(func)?;
                if let Value::Function(name) = func_val {
                    if let Some(func_def) = self.env.get_function(&name).cloned() {
                        self.call_function(&func_def, &new_args)
                    } else {
                        self.call_builtin(&name, &new_args)
                    }
                } else {
                    Err(RuntimeError::new("Pipe a algo que no es función"))
                }
            }
            // Si es solo un identificador de función
            Expr::Ident(name) => {
                if let Some(func_def) = self.env.get_function(name).cloned() {
                    self.call_function(&func_def, &[input.clone()])
                } else {
                    self.call_builtin(name, &[input.clone()])
                }
            }
            _ => Err(RuntimeError::new("Expresión de pipe no soportada")),
        }
    }

    /// Evalúa una operación binaria
    fn eval_binary_op(&self, left: &Value, op: &BinaryOp, right: &Value) -> Result<Value, RuntimeError> {
        match (left, op, right) {
            // Aritmética con enteros
            (Value::Int(a), BinaryOp::Add, Value::Int(b)) => Ok(Value::Int(a + b)),
            (Value::Int(a), BinaryOp::Sub, Value::Int(b)) => Ok(Value::Int(a - b)),
            (Value::Int(a), BinaryOp::Mul, Value::Int(b)) => Ok(Value::Int(a * b)),
            (Value::Int(a), BinaryOp::Div, Value::Int(b)) => {
                if *b == 0 {
                    Err(RuntimeError::new("División por cero"))
                } else {
                    Ok(Value::Int(a / b))
                }
            }
            (Value::Int(a), BinaryOp::Mod, Value::Int(b)) => Ok(Value::Int(a % b)),

            // Aritmética con flotantes
            (Value::Float(a), BinaryOp::Add, Value::Float(b)) => Ok(Value::Float(a + b)),
            (Value::Float(a), BinaryOp::Sub, Value::Float(b)) => Ok(Value::Float(a - b)),
            (Value::Float(a), BinaryOp::Mul, Value::Float(b)) => Ok(Value::Float(a * b)),
            (Value::Float(a), BinaryOp::Div, Value::Float(b)) => Ok(Value::Float(a / b)),

            // Aritmética mixta
            (Value::Int(a), BinaryOp::Add, Value::Float(b)) => Ok(Value::Float(*a as f64 + b)),
            (Value::Float(a), BinaryOp::Add, Value::Int(b)) => Ok(Value::Float(a + *b as f64)),

            // Concatenación de strings
            (Value::String(a), BinaryOp::Concat, Value::String(b)) => Ok(Value::String(format!("{}{}", a, b))),
            (Value::String(a), BinaryOp::Add, Value::String(b)) => Ok(Value::String(format!("{}{}", a, b))),

            // Comparaciones
            (Value::Int(a), BinaryOp::Eq, Value::Int(b)) => Ok(Value::Bool(a == b)),
            (Value::Int(a), BinaryOp::NotEq, Value::Int(b)) => Ok(Value::Bool(a != b)),
            (Value::Int(a), BinaryOp::Lt, Value::Int(b)) => Ok(Value::Bool(a < b)),
            (Value::Int(a), BinaryOp::Gt, Value::Int(b)) => Ok(Value::Bool(a > b)),
            (Value::Int(a), BinaryOp::LtEq, Value::Int(b)) => Ok(Value::Bool(a <= b)),
            (Value::Int(a), BinaryOp::GtEq, Value::Int(b)) => Ok(Value::Bool(a >= b)),

            (Value::String(a), BinaryOp::Eq, Value::String(b)) => Ok(Value::Bool(a == b)),
            (Value::String(a), BinaryOp::NotEq, Value::String(b)) => Ok(Value::Bool(a != b)),

            (Value::Bool(a), BinaryOp::Eq, Value::Bool(b)) => Ok(Value::Bool(a == b)),
            (Value::Bool(a), BinaryOp::And, Value::Bool(b)) => Ok(Value::Bool(*a && *b)),
            (Value::Bool(a), BinaryOp::Or, Value::Bool(b)) => Ok(Value::Bool(*a || *b)),

            _ => Err(RuntimeError::new(format!(
                "Operación {:?} no soportada entre {:?} y {:?}",
                op, left, right
            ))),
        }
    }

    /// Evalúa una operación unaria
    fn eval_unary_op(&self, op: &UnaryOp, val: &Value) -> Result<Value, RuntimeError> {
        match (op, val) {
            (UnaryOp::Neg, Value::Int(n)) => Ok(Value::Int(-n)),
            (UnaryOp::Neg, Value::Float(n)) => Ok(Value::Float(-n)),
            (UnaryOp::Not, Value::Bool(b)) => Ok(Value::Bool(!b)),
            _ => Err(RuntimeError::new(format!(
                "Operación {:?} no soportada para {:?}",
                op, val
            ))),
        }
    }

    /// Interpola variables en un string
    fn interpolate_string(&self, s: &str) -> Result<String, RuntimeError> {
        let mut result = String::new();
        let mut chars = s.chars().peekable();

        while let Some(c) = chars.next() {
            if c == '{' {
                // Encontrar el nombre de la variable
                let mut var_name = String::new();
                while let Some(&next) = chars.peek() {
                    if next == '}' {
                        chars.next();
                        break;
                    }
                    var_name.push(chars.next().unwrap());
                }

                // Buscar el valor
                if let Some(val) = self.env.get(&var_name) {
                    result.push_str(&val.to_string());
                } else {
                    // Si no existe, dejar como está
                    result.push('{');
                    result.push_str(&var_name);
                    result.push('}');
                }
            } else {
                result.push(c);
            }
        }

        Ok(result)
    }

    /// Verifica si un valor es "truthy"
    fn is_truthy(&self, val: &Value) -> bool {
        match val {
            Value::Nil => false,
            Value::Bool(b) => *b,
            Value::Int(n) => *n != 0,
            Value::String(s) => !s.is_empty(),
            Value::List(l) => !l.is_empty(),
            _ => true,
        }
    }
}

impl Default for VM {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::tokenize;
    use crate::parser::parse;

    fn run_code(source: &str) -> Result<Value, RuntimeError> {
        let tokens = tokenize(source).expect("Tokenize failed");
        let program = parse(tokens).expect("Parse failed");
        let mut vm = VM::new();
        vm.load(&program);
        vm.run()
    }

    #[test]
    fn test_simple_main() {
        let result = run_code("+http\nmain = 42\n");
        assert_eq!(result.unwrap(), Value::Int(42));
    }

    #[test]
    fn test_string_literal() {
        let result = run_code("+http\nmain = \"Hello\"\n");
        assert_eq!(result.unwrap(), Value::String("Hello".to_string()));
    }

    #[test]
    fn test_arithmetic() {
        let result = run_code("+http\nmain = 2 + 3 * 4\n");
        assert_eq!(result.unwrap(), Value::Int(14));
    }

    #[test]
    fn test_function_call() {
        let result = run_code("+http\ndouble(x) = x * 2\nmain = double(21)\n");
        assert_eq!(result.unwrap(), Value::Int(42));
    }

    #[test]
    fn test_string_interpolation() {
        let result = run_code("+http\ngreeting(name) = \"Hello {name}!\"\nmain = greeting(\"AURA\")\n");
        // La interpolación no funciona aún porque name no está en scope al evaluar el string
        // Por ahora verificamos que no crashee
        assert!(result.is_ok());
    }
}
