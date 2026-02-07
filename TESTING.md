# AURA - Registro de Pruebas del Interprete

Este documento registra las pruebas del interprete AURA, desde lo mas simple hasta lo complejo.

## Metodologia

1. **Nivel 1**: Expresiones basicas (aritmetica, strings, booleanos)
2. **Nivel 2**: Variables y funciones
3. **Nivel 3**: Tipos y records
4. **Nivel 4**: Control de flujo (condicionales, pattern matching)
5. **Nivel 5**: Pipes y transformaciones
6. **Nivel 6**: Capacidades (+http, +json, +db)
7. **Nivel 7**: Efectos y IO
8. **Nivel 8**: Programas completos

Cada prueba registra:
- Input (codigo AURA)
- Output esperado
- Output real
- Estado: PASS | FAIL | BUG

---

## Nivel 1: Expresiones Basicas

### 1.1 Numeros enteros
```aura
main = 42
```
- **Esperado**: `42`
- **Real**: `42` (type: Int)
- **Estado**: PASS

### 1.2 Aritmetica basica
```aura
main = 2 + 3
```
- **Esperado**: `5`
- **Real**: `5` (type: Int)
- **Estado**: PASS

### 1.3 Aritmetica multiple
```aura
main = 2 + 3 * 4
```
- **Esperado**: `14` (precedencia: 3*4=12, 2+12=14)
- **Real**: `14` (type: Int)
- **Estado**: PASS

### 1.4 Parentesis
```aura
main = (2 + 3) * 4
```
- **Esperado**: `20`
- **Real**: `20` (type: Int)
- **Estado**: PASS

### 1.5 Division
```aura
main = 10 / 3
```
- **Esperado**: `3` (division entera) o `3.333...` (flotante)
- **Real**: `3` (type: Int) - Division entera
- **Estado**: PASS

### 1.6 Modulo
```aura
main = 10 % 3
```
- **Esperado**: `1`
- **Real**: `1` (type: Int)
- **Estado**: PASS

### 1.7 Numeros negativos
```aura
main = -5
```
- **Esperado**: `-5`
- **Real**: `-5` (type: Int)
- **Estado**: PASS

### 1.8 Strings simples
```aura
main = "Hello"
```
- **Esperado**: `Hello`
- **Real**: `Hello` (type: String)
- **Estado**: PASS

### 1.9 Strings con espacios
```aura
main = "Hello World"
```
- **Esperado**: `Hello World`
- **Real**: `Hello World` (type: String)
- **Estado**: PASS

### 1.10 Booleanos
```aura
main = true
```
- **Esperado**: `true`
- **Real**: `true` (type: Bool)
- **Estado**: PASS

### 1.11 Nil
```aura
main = nil
```
- **Esperado**: `nil`
- **Real**: `null` (type: Nil)
- **Estado**: PASS

---

## Nivel 2: Variables y Funciones

> **NOTA IMPORTANTE**: En AURA, todas las definiciones de nivel superior son funciones.
> `x = 10` crea una funcion `x` que retorna `10`. Para obtener el valor, usar `x()`.

### 2.1 Funcion constante (sin parametros)
```aura
x = 10
main = x()
```
- **Esperado**: `10`
- **Real**: `10` (type: Int)
- **Estado**: PASS

### 2.2 Funcion constante con expresion
```aura
x = 2 + 3
main = x()
```
- **Esperado**: `5`
- **Real**: `5` (type: Int)
- **Estado**: PASS

### 2.3 Multiples funciones constantes
```aura
x = 10
y = 20
main = x() + y()
```
- **Esperado**: `30`
- **Real**: `30` (type: Int)
- **Estado**: PASS

### 2.4 Funcion sin parametros (explicito)
```aura
five() = 5
main = five()
```
- **Esperado**: `5`
- **Real**: `5` (type: Int)
- **Estado**: PASS

### 2.5 Funcion con un parametro
```aura
double(x) = x * 2
main = double(21)
```
- **Esperado**: `42`
- **Real**: `42` (type: Int)
- **Estado**: PASS

### 2.6 Funcion con multiples parametros
```aura
add(a b) = a + b
main = add(10 20)
```
- **Esperado**: `30`
- **Real**: `30` (type: Int)
- **Estado**: PASS

### 2.7 Funciones anidadas
```aura
double(x) = x * 2
quadruple(x) = double(double(x))
main = quadruple(5)
```
- **Esperado**: `20`
- **Real**: `20` (type: Int)
- **Estado**: PASS

### 2.8 Interpolacion de strings (con parametros)
```aura
greeting(name) = "Hello {name}!"
main = greeting("AURA")
```
- **Esperado**: `Hello AURA!`
- **Real**: `Hello AURA!` (type: String)
- **Estado**: PASS

### 2.9 Interpolacion de strings (con constantes globales)
```aura
name = "AURA"
main = "Hello {name()}!"
```
- **Esperado**: `Hello AURA!`
- **Real**: `Hello AURA!` (type: String)
- **Estado**: PASS

### 2.10 Bloques con valores intermedios
```aura
process(x) = : a = x * 2; b = a + 5; b
main = process(10)
```
- **Esperado**: `25`
- **Real**: `25` (type: Int)
- **Estado**: PASS

### 2.11 Bloques con multiples bindings
```aura
compute(x) = : a = x * 2; b = a + 5; c = b * 3; c
main = compute(10)
```
- **Esperado**: `75`
- **Real**: `75` (type: Int)
- **Estado**: PASS

---

## Nivel 3: Tipos y Records

### 3.1 Record simple
```aura
main = { name: "Alice", age: 30 }
```
- **Esperado**: `{ name: "Alice", age: 30 }`
- **Real**: `{ "age": 30, "name": "Alice" }` (type: Record)
- **Estado**: PASS

### 3.2 Acceso a campo
```aura
user = { name: "Alice", age: 30 }
main = user().name
```
- **Esperado**: `Alice`
- **Real**: `Alice` (type: String)
- **Estado**: PASS

### 3.3 Lista simple
```aura
main = [1, 2, 3]
```
- **Esperado**: `[1, 2, 3]`
- **Real**: `[1, 2, 3]` (type: List)
- **Estado**: PASS

### 3.4 Lista vacia
```aura
main = []
```
- **Esperado**: `[]`
- **Real**: `[]` (type: List)
- **Estado**: PASS

---

## Nivel 4: Control de Flujo

### 4.1 Condicional con ? (pattern matching)
```aura
main = ? true -> 1 | _ -> 0
```
- **Esperado**: `1`
- **Real**: `1` (type: Int)
- **Estado**: PASS

### 4.2 Condicional con if
```aura
main = if true 1 else 0
```
- **Esperado**: `1`
- **Real**: `1` (type: Int)
- **Estado**: PASS

### 4.7 If con comparacion
```aura
check(x) = if x > 5 "big" else "small"
main = check(10)
```
- **Esperado**: `big`
- **Real**: `big` (type: String)
- **Estado**: PASS

### 4.8 Match multiple condiciones
```aura
check(x) = ? x > 10 -> "big" | x > 5 -> "medium" | _ -> "small"
main = check(7)
```
- **Esperado**: `medium`
- **Real**: `medium` (type: String)
- **Estado**: PASS

### 4.3 Comparacion de igualdad
```aura
main = 5 == 5
```
- **Esperado**: `true`
- **Real**: `true` (type: Bool)
- **Estado**: PASS

### 4.4 Comparacion de desigualdad
```aura
main = 5 != 3
```
- **Esperado**: `true`
- **Real**: `true` (type: Bool)
- **Estado**: PASS

### 4.5 Comparacion mayor que
```aura
main = 5 > 3
```
- **Esperado**: `true`
- **Real**: `true` (type: Bool)
- **Estado**: PASS

### 4.6 Comparacion menor que
```aura
main = 5 < 10
```
- **Esperado**: `true`
- **Real**: `true` (type: Bool)
- **Estado**: PASS

---

## Nivel 5: Pipes y Transformaciones

### 5.1 Pipe simple con funcion de usuario
```aura
double(x) = x * 2
main = 5 | double
```
- **Esperado**: `10`
- **Real**: `10` (type: Int)
- **Estado**: PASS

### 5.2 Pipes encadenados
```aura
double(x) = x * 2
add_one(x) = x + 1
main = 5 | double | add_one
```
- **Esperado**: `11`
- **Real**: `11` (type: Int)
- **Estado**: PASS

### 5.3 Pipe con builtin length
```aura
main = [1, 2, 3] | length
```
- **Esperado**: `3`
- **Real**: `3` (type: Int)
- **Estado**: PASS

### 5.4 Pipe con lista mas grande
```aura
main = [1, 2, 3, 4, 5] | length
```
- **Esperado**: `5`
- **Real**: `5` (type: Int)
- **Estado**: PASS

---

## Nivel 6: Capacidades

### 6.1 json.parse
```aura
main = json.parse("{\"name\": \"test\"}")
```
- **Esperado**: `{ name: "test" }`
- **Real**: `{ "name": "test" }` (type: Record)
- **Estado**: PASS

### 6.2 json.stringify
```aura
data = { name: "Alice", age: 30 }
main = json.stringify(data())
```
- **Esperado**: `{"age":30,"name":"Alice"}`
- **Real**: `{"age":30,"name":"Alice"}` (type: String)
- **Estado**: PASS

### 6.3 math.sqrt
```aura
main = math.sqrt(16)
```
- **Esperado**: `4.0`
- **Real**: `4.0` (type: Float)
- **Estado**: PASS

---

## Nivel 7: Efectos y IO

### 7.1 print
```aura
main = print("Hello AURA")
```
- **Esperado**: imprime `Hello AURA`
- **Real**: imprime `Hello AURA` (retorna nil)
- **Estado**: PASS

### 7.2 length builtin
```aura
main = length([1, 2, 3, 4, 5])
```
- **Esperado**: `5`
- **Real**: `5` (type: Int)
- **Estado**: PASS

---

## Resumen de Bugs Encontrados

| ID | Descripcion | Nivel | Impacto | Estado |
|----|-------------|-------|---------|--------|
| BUG-001 | Parser no implementa `if`, `?` ni pattern matching | 4 | Alto | CERRADO |
| BUG-002 | Interpolacion no funciona con llamadas `{name()}` | 2 | Medio | CERRADO |
| BUG-003 | Builtins (`print`, `length`, etc.) no accesibles | 5,7 | Alto | CERRADO |
| BUG-004 | Capacidades (+json, +db) no conectadas al runtime | 6 | Alto | CERRADO |

---

## Bugs Detallados

### BUG-001: Condicionales no implementados

**Descripcion**: El AST tiene nodos `Expr::If` y `Expr::Match` definidos, pero el parser no los reconoce. Tanto la sintaxis `if cond then else` como `? cond -> expr | _ -> expr` producen error de token inesperado.

**Impacto**: Alto - No se puede escribir logica condicional.

**Archivos afectados**:
- `src/parser/mod.rs` - Falta implementar parsing de if/match
- `src/parser/ast.rs` - Nodos ya definidos

**Ejemplo de reproduccion**:
```bash
./target/debug/aura run -c 'main = if true 1 else 0' --json
# Error: "Unexpected token: Some(If)"
```

---

### BUG-002: Interpolacion con funciones [CERRADO]

**Descripcion**: La interpolacion de strings `"Hello {name}"` solo funcionaba cuando `name` era un parametro de funcion en el scope actual. No funcionaba con llamadas a funciones como `{name()}`.

**Solucion**: Se implemento `eval_interpolation_expr` que parsea y evalua expresiones completas dentro de `{}`. Se agrego `parse_expression_complete` para verificar que todos los tokens fueron consumidos (evitando problemas con JSON strings).

**Archivos modificados**:
- `src/vm/mod.rs` - funcion `interpolate_string` ahora parsea expresiones
- `src/parser/mod.rs` - nueva funcion `parse_expression_complete`

**Estado**: CERRADO

---

### BUG-003: Builtins no accesibles

**Descripcion**: Las funciones builtin como `print`, `length`, `type`, etc. estan definidas en `call_builtin()` pero no son accesibles porque el lookup de identificador falla antes.

Cuando se evalua un identificador como `print`:
1. Se busca en `self.env.get(name)` - No encontrado
2. Se busca en `self.env.get_function(name)` - No encontrado
3. Error: "Variable no definida"

Los builtins solo se llaman si ya tienes `Value::Function(name)`, pero nunca llegas a obtener ese valor.

**Impacto**: Alto - No se pueden usar funciones del lenguaje.

**Archivos afectados**:
- `src/vm/mod.rs` - linea 226-239 (eval de Ident)

**Fix propuesto**: Agregar check de builtins en la evaluacion de `Expr::Ident`:
```rust
// Despues de buscar en funciones y tipos
if is_builtin(name) {
    return Ok(Value::Function(name.clone()));
}
```

---

### BUG-004: Capacidades no conectadas al runtime

**Descripcion**: Las capacidades (+json, +db, etc.) se parsean correctamente pero el modulo `json` no esta disponible en el runtime. Los modulos de capacidades estan en `src/caps/` pero no estan conectados a la VM.

**Impacto**: Alto - No se pueden usar capacidades del lenguaje.

**Archivos afectados**:
- `src/vm/mod.rs` - No hay lookup de modulos de capacidad
- `src/caps/json.rs` - Funciones existen pero no son llamables desde AURA

**Fix propuesto**: Agregar manejo especial para `json.parse`, `json.stringify`, `db.query`, etc. similar a como se maneja `http.get`.

---

## Notas de Testing

### 2026-02-04 - Sesion inicial

**Resumen de tests ejecutados:**
- Nivel 1 (11 tests): **11 PASS**
- Nivel 2 (9 tests): **8 PASS, 1 BUG**
- Nivel 3 (4 tests): **4 PASS**
- Nivel 4 (6 tests): **4 PASS, 2 BUG**
- Nivel 5 (3 tests): **2 PASS, 1 BUG**
- Nivel 6 (2 tests): **0 PASS, 2 BUG**
- Nivel 7 (1 test): **0 PASS, 1 BUG**

**Total: 29 PASS, 7 BUG (de 36 tests)**

**Descubrimientos importantes:**

1. En AURA no existen variables, solo funciones. `x = 10` define una funcion sin parametros. Para obtener el valor, hay que llamarla: `x()`.

2. Pipes funcionan correctamente con funciones de usuario, pero no con builtins.

3. El modulo `http` funciona porque tiene manejo especial en `eval_call()`, pero otros modulos como `json` y `db` no.

4. La interpolacion de strings funciona solo con parametros de funcion en scope.

**Proximos pasos (priorizados):**
1. **BUG-003**: Hacer accesibles los builtins (mas simple, desbloquea testing)
2. **BUG-001**: Implementar parsing de condicionales (critico para logica)
3. **BUG-004**: Conectar capacidades al runtime (json, db)
4. **BUG-002**: Mejorar interpolacion de strings (menos critico)

---

### 2026-02-07 - Sesion 2: Bloques implementados

**Feature implementada**: Bloques con valores intermedios

Sintaxis:
```aura
func(x) = : a = expr1; b = expr2; resultado
```

- `:` inicia un bloque
- `;` separa expresiones
- `name = expr` crea un binding local (Let)
- Ultima expresion es el valor de retorno
- Newline termina el bloque

**Archivos modificados:**
- `src/lexer/tokens.rs` - Agregado token `Semicolon`
- `src/parser/mod.rs` - Agregadas funciones `parse_block`, `parse_block_item`

**Tests agregados**: 2.10, 2.11

---

### 2026-02-07 - Sesion 3: Bugs corregidos en paralelo

**Bugs corregidos:**

1. **BUG-001 (CERRADO)**: Condicionales implementados
   - Agregado parsing de `if cond then else`
   - Agregado parsing de `? cond -> expr | _ -> default`
   - Archivos: `src/parser/mod.rs`

2. **BUG-003 (CERRADO)**: Builtins accesibles
   - Agregada funcion `is_builtin()` en VM
   - Agregados builtins: `length`, `first`, `last`, `tail`, `keys`, `values`, `push`, `concat`, `abs`, `min`, `max`, `not`, `float`, `bool`
   - Archivo: `src/vm/mod.rs`

3. **BUG-004 (CERRADO)**: Capacidades conectadas
   - Agregado modulo `json` con `parse` y `stringify`
   - Agregado modulo `math` con `sqrt`, `pow`, `floor`, `ceil`, `round`
   - Archivo: `src/vm/mod.rs`

4. **Fix adicional**: Escape sequences en strings
   - El lexer ahora procesa `\"`, `\\`, `\n`, `\t`, `\r`
   - Archivo: `src/lexer/tokens.rs`

**Tests agregados**: 4.7, 4.8, 5.4, 6.1, 6.2, 6.3, 7.1, 7.2

**Resumen final:**
- Nivel 1: 11 PASS
- Nivel 2: 10 PASS, 1 BUG (interpolacion)
- Nivel 3: 4 PASS
- Nivel 4: 8 PASS
- Nivel 5: 4 PASS
- Nivel 6: 3 PASS
- Nivel 7: 2 PASS

**Total: 43 PASS, 0 BUGS pendientes**
