# v3 - Especificación del Runtime

## Arquitectura general

```
┌──────────────────────────────────────────────────────────────────┐
│                         AURA RUNTIME                             │
│                                                                  │
│  ┌──────────┐   ┌──────────┐   ┌──────────┐   ┌──────────┐     │
│  │  Source  │──▶│  Parser  │──▶│   AST    │──▶│   IR     │     │
│  │  (.aura) │   │  (Rust)  │   │          │   │          │     │
│  └──────────┘   └──────────┘   └──────────┘   └────┬─────┘     │
│                                                     │            │
│                      ┌──────────────────────────────┤            │
│                      │                              │            │
│                      ▼                              ▼            │
│               ┌──────────┐                   ┌──────────┐       │
│               │    VM    │                   │   LLVM   │       │
│               │ (interp) │                   │ (compile)│       │
│               └────┬─────┘                   └────┬─────┘       │
│                    │                              │              │
│                    └──────────────┬───────────────┘              │
│                                   │                              │
│                                   ▼                              │
│                            ┌──────────┐                          │
│                            │ Executor │                          │
│                            └────┬─────┘                          │
│                                 │                                │
│         ┌───────────────────────┼───────────────────────┐       │
│         │                       │                       │       │
│         ▼                       ▼                       ▼       │
│   ┌──────────┐           ┌──────────┐           ┌──────────┐   │
│   │   HTTP   │           │    DB    │           │  Agent   │   │
│   │ Runtime  │           │ Runtime  │           │  Bridge  │   │
│   └──────────┘           └──────────┘           └──────────┘   │
│                                                                  │
└──────────────────────────────────────────────────────────────────┘
```

## Componentes principales

### 1. Parser

**Tecnología:** logos (lexer) + chumsky (parser)

**Características:**
- Parseo incremental (valida mientras el agente genera)
- Errores estructurados en JSON
- Formato canónico enforced (rechaza código mal formateado)

**Output:** AST tipado

### 2. Type Checker

**Características:**
- Inferencia de tipos Hindley-Milner extendida
- Tipos opcionales explícitos para documentación
- Efectos trackeados en el sistema de tipos

**Tipos de efectos:**
```
Pure    = sin efectos
IO      = input/output (archivos, red, etc.)
DB      = acceso a base de datos
Async   = operación asíncrona
Agent   = requiere interacción con agente
```

### 3. IR (Intermediate Representation)

**Diseño:**
- SSA (Static Single Assignment)
- Optimizaciones básicas: constant folding, dead code elimination
- Preparado para múltiples backends

### 4. Backends

**Fase 1: VM interpretada**
- Stack-based virtual machine
- Rápida iteración durante desarrollo
- Hot reload nativo

**Fase 2: LLVM**
- Compilación AOT para producción
- Optimizaciones agresivas
- Target: binario nativo

**Fase 3: WASM**
- Para ejecutar en browsers/edge
- Sandbox de seguridad

## Sistema de capacidades

Cada capacidad (`+nombre`) activa un módulo del runtime:

### +http
```rust
// Funciones disponibles
http.get!(url) -> Response
http.post!(url body) -> Response
http.put!(url body) -> Response
http.delete!(url) -> Response
http.request!(method url opts) -> Response

// Response
Response {
  status: i
  headers: {s:s}
  body: s
  json(T) -> T?
  text() -> s
}
```

### +json
```rust
json.parse(s) -> any
json.stringify(any) -> s
any.json(T) -> T?
T.to_json() -> s
```

### +db
```rust
// Auto-generado para cada @Type
Type.create!(data) -> Type
Type.get!(id) -> Type?
Type.list!(filters?) -> [Type]
Type.update!(id data) -> Type
Type.delete!(id) -> b
Type.query!() -> QueryBuilder

// QueryBuilder
.where(condition)
.order(field direction?)
.limit(n)
.offset(n)
.include(relation)
.exec!() -> [Type]
```

### +auth
```rust
auth.token!(user) -> s
auth.verify!(token) -> User?
auth.hash(password) -> s
auth.check(password hash) -> b

// Variables especiales en contexto API
@me -> User          // Usuario autenticado
@token -> s          // Token actual
```

### +ws
```rust
ws.broadcast!(channel data)
ws.send!(client_id data)
ws.subscribe!(channel handler)
```

### +fs
```rust
fs.read!(path) -> s
fs.write!(path content)
fs.append!(path content)
fs.delete!(path)
fs.exists!(path) -> b
fs.list!(path) -> [s]
```

### +crypto
```rust
crypto.hash(algorithm data) -> s
crypto.hmac(key data) -> s
crypto.encrypt(key data) -> s
crypto.decrypt(key data) -> s
crypto.random(bytes) -> s
```

### +time
```rust
now() -> ts
today() -> date
time.parse(s format?) -> ts
time.format(ts format) -> s
ts.add(duration) -> ts
ts.diff(ts) -> duration

// Duraciones
1.second / 1.seconds
1.minute / 1.minutes
1.hour / 1.hours
1.day / 1.days
1.week / 1.weeks
```

### +email
```rust
email.send!(to subject body opts?)
email.template!(to template data)
```

## Hot Reload

El runtime soporta actualización en caliente:

```ruby
+append                          # Modo expansión

# Nuevas definiciones se agregan sin reiniciar
@NewType { ... }
new_function(x) = ...
```

**Proceso:**
1. Parser valida nuevo código
2. Type checker verifica compatibilidad
3. Nuevo código se compila a IR
4. VM incorpora nuevas definiciones
5. Estado existente se preserva

**Restricciones:**
- No se pueden eliminar tipos en uso
- No se pueden cambiar firmas de funciones en uso
- Cambios incompatibles requieren reinicio

## Manejo de errores

### Errores estructurados

Todos los errores son JSON:

```json
{
  "code": "E001",
  "severity": "error",
  "location": {
    "file": "main.aura",
    "line": 15,
    "col": 8,
    "end_col": 12
  },
  "message": "Tipo incompatible",
  "details": {
    "expected": "User",
    "got": "s"
  },
  "suggestion": {
    "message": "Usar .json(User) para convertir",
    "replacement": ".json(User)"
  },
  "context": [
    {"line": 14, "code": "  data = http.get!(url)"},
    {"line": 15, "code": "  user:User = data", "highlight": true},
    {"line": 16, "code": "  process(user)"}
  ]
}
```

### Códigos de error

| Código | Categoría |
|--------|-----------|
| E0xx | Sintaxis |
| E1xx | Tipos |
| E2xx | Referencias (variable no encontrada, etc.) |
| E3xx | Efectos (efecto no manejado, etc.) |
| E4xx | Runtime (null, índice fuera de rango, etc.) |
| E5xx | Capacidades (capacidad no habilitada, etc.) |
| E9xx | Agente (errores del bridge) |

## Introspección

El runtime expone información para agentes:

```ruby
?types                           # Lista todos los tipos
?funcs                           # Lista todas las funciones
?caps                            # Capacidades activas
?type User                       # Detalle de User
?func fetch                      # Detalle de fetch
?deps fetch                      # Dependencias de fetch
?errors                          # Errores recientes
?state                           # Estado del runtime
```

## Modelo de ejecución

### Secuencial por defecto

```ruby
main! = :
  a = fetch!(1)                  # Espera
  b = fetch!(2)                  # Espera
  process(a b)
```

### Concurrencia explícita

```ruby
main! = :
  [a b] = parallel!(fetch!(1) fetch!(2))    # Concurrente
  process(a b)
```

### Streams

```ruby
process_stream!(ids) = :
  ids
  | stream
  | map!(fetch!)
  | filter(_.active)
  | each!(save!)
```

## Configuración del runtime

```ruby
+runtime(
  env: "development" | "production"
  port: 3000
  host: "0.0.0.0"
  db: "postgres://..."
  log: .debug | .info | .warn | .error
  cors: ["*"] | ["domain.com"]
  limits: {
    request_size: 10.mb
    rate: 100.per_minute
  }
)
```

## Próximo

v4-agente-bridge.md: Protocolo de comunicación runtime ↔ agente IA.
