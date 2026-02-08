# AURA - Referencia de Sintaxis

## Quick Reference

```ruby
# Capacidades e Imports
+http +json +db            # capacidades builtin
+utils                     # importa utils.aura

# Intención y constraints
goal "descripción"                        # intención para healing
invariant api_url != "https://prod.com"   # constraint que healing respeta
expect result > 0 "debe ser positivo"     # verificación de intención

# Funciones
x = 42                     # constante (función sin parámetros)
double(n) = n * 2          # función con parámetro
add(a b) = a + b           # múltiples parámetros (sin coma)
greet(name) = "Hola {name}"  # interpolación

# Self-healing automático
@self_heal
main = risky_operation()

@self_heal(max_attempts: 5, mode: "technical")
get_data() = http.get(url)

# Bloques
process(x) = : a = x * 2; b = a + 10; b

# Condicionales
abs(n) = if n < 0 (-n) else n
max(a b) = if a > b a else b

# Pattern Matching
classify(n) = ? n == 0 -> "zero" | n > 0 -> "positive" | _ -> "negative"

# Pipes
result = data |> transform |> filter |> save

# Records
user = {name: "Alice", age: 30}
user.name                  # acceso

# Listas
nums = [1, 2, 3, 4, 5]
first(nums)                # primer elemento

# Operadores
a + b  a - b  a * b  a / b  a % b    # aritméticos
a == b  a != b  a < b  a > b         # comparación
a ++ b                               # concatenación
a ?? b                               # null coalescing
a?.field                             # safe navigation

# Variables de entorno
+env
api_key = env.get("API_KEY")
db_url = env.get("DB_URL", "sqlite:./default.db")
```

---

## EBNF Formal

```ebnf
(* ═══════════════════════════════════════════════════════════════ *)
(*                           PROGRAMA                               *)
(* ═══════════════════════════════════════════════════════════════ *)

program      = { capability } { definition } ;

capability   = "+" identifier ;

definition   = goal_def
             | type_def
             | func_def ;

goal_def     = "goal" string ;

(* ═══════════════════════════════════════════════════════════════ *)
(*                        DEFINICIÓN DE TIPOS                       *)
(* ═══════════════════════════════════════════════════════════════ *)

type_def     = "@" identifier "{" { field } "}" ;

field        = identifier ":" type { annotation } ;

type         = ":i"                    (* Int *)
             | ":f"                    (* Float *)
             | ":s"                    (* String *)
             | ":b"                    (* Bool *)
             | ":ts"                   (* Timestamp *)
             | ":uuid"                 (* UUID *)
             | "[" type "]"            (* List *)
             | identifier              (* Named type *)
             | type "?"                (* Optional *)
             ;

annotation   = "@pk" | "@unique" | "@email" | "@url" | "@hash"
             | "@hide" | "@auto" | "@rel" | "@index"
             | "@min" "(" expression ")"
             | "@max" "(" expression ")"
             ;

(* ═══════════════════════════════════════════════════════════════ *)
(*                      DEFINICIÓN DE FUNCIONES                     *)
(* ═══════════════════════════════════════════════════════════════ *)

func_def     = identifier [ "(" parameters ")" ] [ "!" ] "=" expression ;

parameters   = identifier { identifier } ;

(* ═══════════════════════════════════════════════════════════════ *)
(*                          EXPRESIONES                             *)
(* ═══════════════════════════════════════════════════════════════ *)

expression   = pipe_expr ;

pipe_expr    = comparison { "|>" comparison } ;

comparison   = additive { comp_op additive } ;
comp_op      = "==" | "!=" | "<" | ">" | "<=" | ">=" ;

additive     = multiplicative { add_op multiplicative } ;
add_op       = "+" | "-" | "++" ;

multiplicative = unary { mul_op unary } ;
mul_op       = "*" | "/" | "%" ;

unary        = "-" unary
             | "!" unary
             | call ;

call         = primary { call_suffix } ;
call_suffix  = "(" [ arguments ] ")"
             | "!" "(" [ arguments ] ")"
             | "." identifier
             | "?." identifier
             ;

arguments    = expression { "," expression } ;

primary      = integer
             | float
             | string
             | "true" | "false" | "nil"
             | identifier
             | "(" expression ")"
             | list_expr
             | record_expr
             | block_expr
             | if_expr
             | match_expr
             ;

(* ═══════════════════════════════════════════════════════════════ *)
(*                      EXPRESIONES COMPUESTAS                      *)
(* ═══════════════════════════════════════════════════════════════ *)

list_expr    = "[" [ expression { "," expression } ] "]" ;

record_expr  = "{" [ field_init { "," field_init } ] "}" ;
field_init   = identifier ":" expression ;

block_expr   = ":" statement { ";" statement } ;
statement    = identifier "=" expression
             | expression ;

if_expr      = "if" expression expression "else" expression ;

match_expr   = "?" match_arm { "|" match_arm } ;
match_arm    = pattern "->" expression ;
pattern      = "_"
             | expression ;

(* ═══════════════════════════════════════════════════════════════ *)
(*                           LITERALES                              *)
(* ═══════════════════════════════════════════════════════════════ *)

integer      = digit { digit } ;
float        = digit { digit } "." digit { digit } ;
string       = '"' { char | escape | interpolation } '"' ;
interpolation = "{" expression "}" ;
escape       = "\" ( "n" | "t" | "r" | "\" | '"' ) ;
identifier   = letter { letter | digit | "_" } ;

digit        = "0" | "1" | ... | "9" ;
letter       = "a" | ... | "z" | "A" | ... | "Z" | "_" ;
```

---

## Capacidades Builtin

| Capacidad | Funciones | Descripción |
|-----------|-----------|-------------|
| `+http` | `http.get`, `http.post`, `http.put`, `http.delete` | Cliente HTTP |
| `+json` | `json.parse`, `json.stringify` | Serialización JSON |
| `+db` | `db.connect`, `db.query`, `db.execute` | Base de datos SQL |
| `+env` | `env.get`, `env.set`, `env.exists` | Variables de entorno |
| `+math` | `sqrt`, `pow`, `sin`, `cos`, `log` | Matemáticas |
| `+time` | `time.now`, `time.format`, `time.parse` | Tiempo |
| `+crypto` | `crypto.hash`, `crypto.hmac` | Criptografía |

Si `+nombre` no es builtin, se busca `nombre.aura` en el directorio actual.

---

## Tipos Primitivos

| Sintaxis | Tipo | Ejemplo |
|----------|------|---------|
| `:i` | Integer | `42` |
| `:f` | Float | `3.14` |
| `:s` | String | `"hello"` |
| `:b` | Boolean | `true`, `false` |
| `:ts` | Timestamp | `time.now()` |
| `:uuid` | UUID | Auto-generado |
| `[T]` | List | `[1, 2, 3]` |
| `T?` | Optional | Puede ser `nil` |

---

## Anotaciones

| Anotación | Uso | Descripción |
|-----------|-----|-------------|
| `@pk` | Campo | Primary key |
| `@unique` | Campo | Valor único |
| `@email` | Campo | Validación email |
| `@url` | Campo | Validación URL |
| `@hash` | Campo | Almacenar hasheado |
| `@hide` | Campo | No exponer en API |
| `@auto` | Campo | Auto-generar |
| `@rel` | Campo | Relación FK |
| `@index` | Campo | Crear índice |
| `@min(n)` | Campo | Valor mínimo |
| `@max(n)` | Campo | Valor máximo |

---

## Precedencia de Operadores

| Precedencia | Operadores | Asociatividad |
|-------------|------------|---------------|
| 1 (menor) | `\|>` | Izquierda |
| 2 | `== != < > <= >=` | Izquierda |
| 3 | `+ - ++` | Izquierda |
| 4 | `* / %` | Izquierda |
| 5 | `- !` (unarios) | Derecha |
| 6 (mayor) | `.` `?.` `()` | Izquierda |

---

## Ejemplos

### API Client
```ruby
+http +json

goal "obtener y mostrar usuario de la API"

get_user(id) = : r = http.get("https://api.com/users/{id}"); json.parse(r.body)
main = : user = get_user(1); "User: {user.name}"
```

### CRUD con Tipos
```ruby
+db

@User {
    id   :i    @pk @auto
    name :s    @min(2) @max(100)
    email :s   @email @unique
}

conn = db.connect("sqlite:./app.db")
get_users = db.query(conn(), "SELECT * FROM users", [])
create_user(name email) = db.execute(conn(), "INSERT INTO users (name, email) VALUES (?, ?)", [name, email])
```

### Modularización
```ruby
# utils.aura
double(n) = n * 2
triple(n) = n * 3

# main.aura
+utils

main = double(21)  # 42
```
