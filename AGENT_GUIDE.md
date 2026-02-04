# AURA - Guía para Agentes IA

Este documento contiene todo lo que un agente IA necesita para entender y trabajar con AURA.

## 1. Qué es AURA

AURA (Agent-Unified Runtime Architecture) es un lenguaje de programación diseñado para agentes IA con estas características:

- **Mínimo** - Menos tokens = menos costo, menos errores
- **No ambiguo** - Una sola forma de hacer cada cosa
- **Self-healing** - Los errores se pueden auto-reparar

## 2. Sintaxis Completa

### 2.1 Capacidades (en lugar de imports)

```aura
+core                    # Funciones básicas (print!, etc.)
+http                    # HTTP: get!, post!, put!, delete!
+json                    # JSON: parse, stringify
+db                      # Database: connect!, query!, execute!
+agent(on_error: .fix)   # Self-healing habilitado
```

### 2.2 Tipos

```aura
# Tipos primitivos
s     # String
i     # Integer
f     # Float
b     # Bool
nil   # Null/None

# Tipos compuestos
[T]       # Lista de T
{K: V}    # Mapa de K a V
T?        # T nullable (puede ser nil)

# Tipos custom (records)
@User {
    id: uuid @pk           # Primary key
    name: s @min(2) @max(100)
    email: s? @email       # Nullable con validación
    age: i @range(0, 150)
    role: Role = .user     # Default value
}

# Enums
@Role = admin | user | guest
@Status = pending | active | inactive
```

### 2.3 Funciones

```aura
# Función pura (sin efectos)
add(a b) = a + b
square(x) = x * x
greet(name) = "Hello {name}!"

# Función con efectos (IO) - usa !
fetch!(url) = http.get!(url)
save!(data) = db.execute!(conn, "INSERT...", data)
log!(msg) = print!(msg)

# Función sin parámetros
now!() = time.now!()
main = print!("Hello")

# Función multilínea
process(items) = {
    filtered = items | filter(_.active)
    mapped = filtered | map(_.value)
    mapped | sum
}
```

### 2.4 Expresiones

```aura
# Aritméticas
1 + 2, 3 - 1, 4 * 5, 10 / 2, 7 % 3

# Comparación
a == b, a != b, a < b, a > b, a <= b, a >= b

# Lógicas
a && b, a || b, !a

# Pipes (encadenamiento)
data | filter(_.active) | map(_.name) | sort | take(10)

# Pattern matching
result | Ok(value) -> value | Err(e) -> nil

# Null coalescing
user?.name ?? "Anonymous"
config?.timeout ?? 30

# Interpolación de strings
"Hello {name}, you have {count} messages"
"User: {user.name} ({user.email})"

# Condicionales
? condition -> then_value
? a > b -> "greater" | a < b -> "less" | _ -> "equal"

# Listas
[1, 2, 3]
users | map(_.name)

# Records
{ name: "Alice", age: 30 }
user.name
user?.address?.city
```

### 2.5 Efectos y Pureza

```aura
# Funciones puras - NO pueden:
# - Hacer IO (red, disco, consola)
# - Modificar estado global
# - Tener efectos secundarios

pure_fn(x) = x * 2  # OK - pura

# Funciones con efectos - DEBEN terminar en !
effect_fn!(x) = {
    print!(x)        # IO
    http.get!(url)   # Red
    db.query!(sql)   # Base de datos
}

# Llamar función con efecto requiere !
main = {
    result = fetch!(url)   # Correcto
    # result = fetch(url)  # ERROR: falta !
}
```

## 3. Capacidades Disponibles

### 3.1 +http

```aura
+http

# GET
response = http.get!(url)
response = http.get!(url, headers: {"Auth": token})

# POST
response = http.post!(url, body: data)
response = http.post!(url, body: json, headers: {"Content-Type": "application/json"})

# PUT
response = http.put!(url, body: data)

# DELETE
response = http.delete!(url)

# Response es un Record con:
# { status: Int, headers: Record, body: String }
```

### 3.2 +json

```aura
+json

# Parse string a valor
data = json.parse('{"name": "Alice"}')

# Stringify valor a string
text = json.stringify(data)
text = json.stringify(data, pretty: true)
```

### 3.3 +db

```aura
+db

# Conectar
conn = db.connect!("sqlite:app.db")
conn = db.connect!(":memory:")

# Query (SELECT)
users = db.query!(conn, "SELECT * FROM users WHERE active = ?", [true])
# Retorna lista de records

# Execute (INSERT/UPDATE/DELETE)
result = db.execute!(conn, "INSERT INTO users (name) VALUES (?)", ["Alice"])
# Retorna { rows_affected: Int, last_insert_id: Int }

# Cerrar
db.close!(conn)
```

## 4. Errores Comunes y Fixes

### E1xx - Errores de Sintaxis

| Código | Error | Causa | Fix |
|--------|-------|-------|-----|
| E101 | Token inesperado | Sintaxis incorrecta | Revisar gramática |
| E102 | Paréntesis no cerrado | Falta ) | Agregar ) |
| E103 | Llave no cerrada | Falta } | Agregar } |

### E2xx - Errores de Referencia

| Código | Error | Causa | Fix |
|--------|-------|-------|-----|
| E201 | Variable no definida | Usar antes de declarar | Declarar primero |
| E202 | Función no definida | Función no existe | Definir función |
| E203 | Tipo no definido | @Type no existe | Definir tipo |

### E3xx - Errores de Tipo

| Código | Error | Causa | Fix |
|--------|-------|-------|-----|
| E301 | Tipo incompatible | Esperaba X, recibió Y | Convertir tipo |
| E302 | Argumento faltante | Menos args de lo esperado | Agregar argumento |
| E303 | Argumento extra | Más args de lo esperado | Remover argumento |

### E4xx - Errores de Efecto

| Código | Error | Causa | Fix |
|--------|-------|-------|-----|
| E401 | Efecto no manejado | Llamar fn! sin ! | Agregar ! a la llamada |
| E402 | Efecto en contexto puro | fn! dentro de fn pura | Marcar fn como ! |

### E5xx - Errores de Capacidad

| Código | Error | Causa | Fix |
|--------|-------|-------|-----|
| E501 | Capacidad no habilitada | Usar http sin +http | Agregar +http |
| E502 | Capacidad desconocida | +foo no existe | Usar capacidad válida |

## 5. Patrones Idiomáticos

### 5.1 Fetch y procesar JSON

```aura
+http +json

fetch_users!() = {
    response = http.get!("https://api.example.com/users")
    ? response.status == 200
        -> json.parse(response.body)
        | _ -> []
}
```

### 5.2 CRUD con base de datos

```aura
+db

@User { id:i name:s email:s }

create_user!(name email) = {
    conn = db.connect!("sqlite:app.db")
    result = db.execute!(conn,
        "INSERT INTO users (name, email) VALUES (?, ?)",
        [name, email])
    db.close!(conn)
    result.last_insert_id
}

get_user!(id) = {
    conn = db.connect!("sqlite:app.db")
    users = db.query!(conn, "SELECT * FROM users WHERE id = ?", [id])
    db.close!(conn)
    users | first ?? nil
}
```

### 5.3 Pipeline de datos

```aura
process_orders(orders) =
    orders
    | filter(_.status == .active)
    | map(o -> { ...o, total: o.quantity * o.price })
    | sort_by(_.total, desc: true)
    | take(10)
```

### 5.4 Manejo de errores

```aura
safe_fetch!(url) =
    http.get!(url)
    | Ok(r) -> json.parse(r.body)
    | Err(e) -> { error: e.message }

# Con default
get_config!(key) =
    config?.get(key) ?? defaults.get(key) ?? nil
```

## 6. Protocolo de Healing

Cuando un agente recibe un error de AURA, debe responder en JSON:

### Request (lo que recibe el agente)

```json
{
    "event_type": "error",
    "context": {
        "source": "x + 1",
        "surrounding_code": "main = {\n  x + 1\n}"
    },
    "location": {
        "file": "main.aura",
        "line": 2,
        "column": 3
    },
    "message": "Variable 'x' no definida",
    "previous_attempts": []
}
```

### Response (lo que debe responder el agente)

```json
{
    "action": "patch",
    "patch": {
        "old_code": "x + 1",
        "new_code": "x = 0\nx + 1"
    },
    "explanation": "La variable 'x' no estaba definida. Se agregó su declaración.",
    "confidence": 0.95
}
```

### Acciones posibles

| Action | Descripción | Campos requeridos |
|--------|-------------|-------------------|
| `patch` | Reemplazar código | `patch.old_code`, `patch.new_code` |
| `generate` | Generar código nuevo | `generated_code` |
| `suggest` | Sugerir opciones | `suggestions[]` |
| `clarify` | Pedir más info | `questions[]` |
| `escalate` | Requiere humano | `escalation_reason` |

### Nivel de confianza

- `confidence >= 0.8` - Auto-aplicar el fix
- `confidence >= 0.5` - Sugerir al usuario
- `confidence < 0.5` - Pedir clarificación

## 7. CLI para Agentes

```bash
# Verificar sintaxis y tipos
aura check file.aura
# Salida: JSON con errores o "OK"

# Ejecutar programa
aura run file.aura
# Salida: resultado o error JSON

# Ver AST (para análisis)
aura parse file.aura --json
# Salida: AST en JSON

# Ver tokens (para debug)
aura lex file.aura --json
# Salida: tokens en JSON

# REPL interactivo
aura repl
# Para pruebas rápidas

# Info del runtime
aura info --json
# Salida: versión, capacidades, etc.
```

## 8. Ejemplos Completos

### 8.1 API Client

```aura
+http +json

@User { id:i name:s email:s }
@ApiResponse { data:[User] total:i }

fetch_users!(page limit) = {
    url = "https://api.example.com/users?page={page}&limit={limit}"
    response = http.get!(url)

    ? response.status == 200 -> {
        parsed = json.parse(response.body)
        parsed.data
    } | _ -> []
}

main = {
    users = fetch_users!(1, 10)
    users | each(u -> print!("User: {u.name}"))
}
```

### 8.2 Web Scraper Simple

```aura
+http +json

scrape!(url) = {
    response = http.get!(url)
    ? response.status == 200
        -> response.body
        | _ -> ""
}

main = {
    html = scrape!("https://example.com")
    print!("Length: {html.length}")
}
```

### 8.3 Database CRUD

```aura
+db +json

@Task { id:i title:s done:b }

init_db!() = {
    conn = db.connect!(":memory:")
    db.execute!(conn, "CREATE TABLE tasks (id INTEGER PRIMARY KEY, title TEXT, done INTEGER)")
    conn
}

add_task!(conn title) =
    db.execute!(conn, "INSERT INTO tasks (title, done) VALUES (?, 0)", [title])

get_tasks!(conn) =
    db.query!(conn, "SELECT * FROM tasks", [])

complete_task!(conn id) =
    db.execute!(conn, "UPDATE tasks SET done = 1 WHERE id = ?", [id])

main = {
    conn = init_db!()

    add_task!(conn, "Learn AURA")
    add_task!(conn, "Build something")

    tasks = get_tasks!(conn)
    tasks | each(t -> print!("{t.id}: {t.title} [{? t.done -> 'x' | _ -> ' '}]"))

    complete_task!(conn, 1)

    db.close!(conn)
}
```

## 9. Anti-patrones (Qué NO hacer)

```aura
# ❌ MAL: Múltiples formas de hacer lo mismo
if condition then x else y      # No existe
condition ? x : y               # No existe

# ✓ BIEN: Una sola forma
? condition -> x | _ -> y

# ❌ MAL: Imports tradicionales
import http
from json import parse

# ✓ BIEN: Capacidades
+http +json

# ❌ MAL: Efectos sin marcar
fetch(url) = http.get(url)      # Error: falta !

# ✓ BIEN: Efectos marcados
fetch!(url) = http.get!(url)

# ❌ MAL: Null checks verbosos
if user != nil && user.address != nil && user.address.city != nil

# ✓ BIEN: Optional chaining
user?.address?.city ?? "Unknown"
```

## 10. Checklist de Validación

Antes de generar código AURA, verificar:

- [ ] ¿Todas las capacidades necesarias están declaradas? (+http, +json, etc.)
- [ ] ¿Las funciones con IO terminan en !?
- [ ] ¿Las llamadas a funciones ! incluyen el !?
- [ ] ¿Los tipos custom están definidos con @?
- [ ] ¿Se usa interpolación {var} en lugar de concatenación?
- [ ] ¿Los valores nullable usan ? y ??
- [ ] ¿Los pipes | fluyen de izquierda a derecha?
- [ ] ¿El pattern matching cubre todos los casos?
