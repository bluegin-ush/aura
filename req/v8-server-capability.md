# V8: Capacidad +server - Servidor HTTP Nativo

## Objetivo

Implementar la capacidad `+server` que permite a AURA actuar como servidor HTTP, manejando requests y responses de forma nativa.

---

## Diseño de API

### Sintaxis Propuesta

```ruby
+server +json +db

# Definir handlers por ruta
GET "/users" = : users = db.query(conn, "SELECT * FROM users", []); json(users)

GET "/users/:id" (id) = : user = db.query(conn, "SELECT * FROM users WHERE id = ?", [id]); json(user)

POST "/users" (body) = : db.execute(conn, "INSERT INTO users (name) VALUES (?)", [body.name]); json({ok: true})

PUT "/users/:id" (id, body) = : db.execute(conn, "UPDATE users SET name = ? WHERE id = ?", [body.name, id]); json({ok: true})

DELETE "/users/:id" (id) = : db.execute(conn, "DELETE FROM users WHERE id = ?", [id]); json({ok: true})

# Iniciar servidor
main = server.listen(8080)
```

### Alternativa más explícita

```ruby
+server +json +db

list_users(req) = : users = db.query(conn, "SELECT * FROM users", []); {status: 200, body: users}

get_user(req) = : id = req.params.id; user = db.query(conn, "SELECT * FROM users WHERE id = ?", [id]); {status: 200, body: user}

create_user(req) = : db.execute(conn, "INSERT INTO users (name) VALUES (?)", [req.body.name]); {status: 201, body: {ok: true}}

routes = [
    {method: "GET", path: "/users", handler: list_users},
    {method: "GET", path: "/users/:id", handler: get_user},
    {method: "POST", path: "/users", handler: create_user}
]

main = server.start(8080, routes)
```

---

## Modelo de Request

```ruby
# El request que recibe cada handler
request = {
    method: "POST",
    path: "/users/123",
    params: {id: "123"},           # Parámetros de ruta
    query: {page: "1", limit: "10"}, # Query string
    headers: {content_type: "application/json"},
    body: {name: "Alice"}          # Body parseado si es JSON
}
```

## Modelo de Response

```ruby
# Respuesta simple
{status: 200, body: {users: [...]}}

# Respuesta con headers
{status: 201, headers: {location: "/users/5"}, body: {id: 5}}

# Helpers
json(data) = {status: 200, headers: {content_type: "application/json"}, body: data}
created(data) = {status: 201, body: data}
not_found(msg) = {status: 404, body: {error: msg}}
error(msg) = {status: 500, body: {error: msg}}
```

---

## Arquitectura de Implementación

```
┌─────────────────────────────────────────────────────────────────┐
│                         AURA RUNTIME                             │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐      │
│  │   +server    │    │    Router    │    │   Handlers   │      │
│  │   (Rust)     │───▶│   (Rust)     │───▶│   (AURA)     │      │
│  └──────────────┘    └──────────────┘    └──────────────┘      │
│         │                   │                    │              │
│         ▼                   ▼                    ▼              │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐      │
│  │  TCP/HTTP    │    │ Path Match   │    │   VM.eval    │      │
│  │  (tokio)     │    │ Param Extract│    │  (existing)  │      │
│  └──────────────┘    └──────────────┘    └──────────────┘      │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### Componentes

1. **HTTP Server (Rust)**: Usa `hyper` o `axum` para TCP/HTTP
2. **Router (Rust)**: Parsea rutas, extrae parámetros
3. **Bridge (Rust↔AURA)**: Convierte Request HTTP → Value AURA
4. **Handlers (AURA)**: Funciones definidas por el usuario
5. **Response Builder (Rust)**: Convierte Value AURA → Response HTTP

---

## Fases de Implementación

### Fase 1: Server Mínimo (MVP)
**Objetivo**: Servidor que responde a requests básicos

```
Semana 1:
├── [ ] HTTP server básico con hyper/axum en Rust
├── [ ] Binding server.listen(port) en AURA
├── [ ] Request simple como Value (method, path, body)
├── [ ] Response simple {status, body}
└── [ ] Un endpoint hardcodeado funcionando
```

**Entregable**:
```ruby
+server
main = server.listen(8080, req -> {status: 200, body: "Hello"})
```

### Fase 2: Routing
**Objetivo**: Múltiples rutas y métodos

```
Semana 2:
├── [ ] Router con path matching
├── [ ] Extracción de parámetros (:id)
├── [ ] Soporte GET, POST, PUT, DELETE
├── [ ] Query string parsing
└── [ ] Headers básicos
```

**Entregable**:
```ruby
+server
routes = [
    {method: "GET", path: "/", handler: home},
    {method: "GET", path: "/users/:id", handler: get_user}
]
main = server.start(8080, routes)
```

### Fase 3: JSON & Middleware
**Objetivo**: Productivo para APIs reales

```
Semana 3:
├── [ ] Body JSON automático
├── [ ] Content-Type handling
├── [ ] Helpers (json, created, not_found, error)
├── [ ] Logging de requests
└── [ ] Error handling graceful
```

**Entregable**:
```ruby
+server +json
get_user(req) = : id = req.params.id; user = find_user(id); json(user)
```

### Fase 4: MotoStock
**Objetivo**: Aplicación real funcionando

```
Semana 4:
├── [ ] Implementar todos los endpoints de MotoStock
├── [ ] Tests de integración
├── [ ] Documentación
└── [ ] Benchmark vs Python/Node
```

---

## Archivos a Crear/Modificar

```
src/
├── caps/
│   └── server.rs          # NUEVO: Capacidad +server
├── server/                 # NUEVO: Módulo servidor
│   ├── mod.rs             # Exports
│   ├── http.rs            # HTTP server (hyper/axum)
│   ├── router.rs          # Router y path matching
│   ├── request.rs         # Request → Value
│   ├── response.rs        # Value → Response
│   └── handlers.rs        # Bridge a AURA VM
├── lexer/
│   └── tokens.rs          # Agregar tokens GET, POST, etc. (opcional)
└── vm/
    └── mod.rs             # Agregar server.listen, server.start
```

---

## Dependencias Rust

```toml
[dependencies]
tokio = { version = "1", features = ["full"] }
hyper = { version = "1", features = ["server", "http1"] }
# O alternativamente:
axum = "0.7"
```

---

## Riesgos y Mitigaciones

| Riesgo | Probabilidad | Impacto | Mitigación |
|--------|--------------|---------|------------|
| Async en AURA VM | Alta | Alto | VM sync, server async con channels |
| Memory leaks en long-running | Media | Alto | Tests de stress, profiling |
| Performance pobre | Baja | Medio | Benchmark temprano, optimizar hot paths |
| Complejidad del router | Baja | Bajo | Usar crate existente (matchit) |

---

## Criterios de Éxito

1. **Funcional**: MotoStock corre 100% en AURA
2. **Performance**: < 5ms latencia p99 para CRUD simple
3. **Estabilidad**: 24h corriendo sin memory leak
4. **DX**: Código AURA limpio y expresivo

---

## Siguiente Paso

Comenzar con Fase 1: Server Mínimo
- Crear `src/server/mod.rs`
- Implementar HTTP server básico
- Conectar con VM
