# v6 - Roadmap Fase 2: Expansión

**Fecha:** 2026-02-04
**Estado:** ✓ COMPLETADO (100%)

## Tracks Paralelos

La Fase 2 se desarrolla en 4 tracks que pueden avanzar en paralelo:

```
┌─────────────────────────────────────────────────────────────────┐
│                        FASE 2: EXPANSIÓN                        │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Track A          Track B          Track C          Track D     │
│  ─────────        ─────────        ─────────        ─────────   │
│  Agent Bridge     Capacidades      Hot Reload       DevEx       │
│                                                                 │
│  ┌─────────┐      ┌─────────┐      ┌─────────┐      ┌────────┐ │
│  │ Proto   │      │ +http   │      │ +append │      │ REPL   │ │
│  │ Request │ ✓    │ reqwest │ ✓    │ mode    │      │ mejor  │ ✓│
│  └────┬────┘      └────┬────┘      └────┬────┘      └───┬────┘ │
│       │                │                │               │       │
│  ┌────▼────┐      ┌────▼────┐      ┌────▼────┐      ┌───▼────┐ │
│  │ Proto   │      │ +json   │      │ Diff    │      │ Errors │ │
│  │ Response│ ✓    │ serde   │      │ Apply   │      │ bonitos│ │
│  └────┬────┘      └────┬────┘      └────┬────┘      └───┬────┘ │
│       │                │                │               │       │
│  ┌────▼────┐      ┌────▼────┐      ┌────▼────┐      ┌───▼────┐ │
│  │ Self    │      │ +db     │      │ State   │      │ LSP    │ │
│  │ Healing │      │ SQLx    │      │ Preserve│      │ básico │ │
│  └─────────┘      └─────────┘      └─────────┘      └────────┘ │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## Track A: Agent Bridge ✓ COMPLETADO

**Objetivo:** Permitir que el runtime se comunique con agentes IA para self-healing y expansión.

### A1. Protocolo de Request ✓
```rust
// src/agent/request.rs - IMPLEMENTADO
pub struct AgentRequest {
    pub event_type: EventType,  // Error, Missing, Performance, Expansion
    pub context: Context,        // código, tipos, estado
    pub constraints: Constraints // max_tokens, timeout, temperature
}

// Builders fluidos:
AgentRequest::error(code, file, line, col)
    .with_message("Variable 'x' no definida")
    .with_constraints(Constraints::strict())
```

### A2. Protocolo de Response ✓
```rust
// src/agent/response.rs - IMPLEMENTADO
pub struct AgentResponse {
    pub action: Action,      // Patch, Generate, Suggest, Clarify, Escalate
    pub patch: Option<Patch>,
    pub explanation: String,
    pub confidence: f32
}

// Factory methods:
AgentResponse::patch(patch, explanation, confidence)
AgentResponse::generate(code, explanation, confidence)
AgentResponse::suggest(suggestions, explanation)
```

### A3. Bridge y Proveedores ✓
```rust
// src/agent/bridge.rs - IMPLEMENTADO
pub trait AgentProvider: Send + Sync {
    fn send_request(&self, req: AgentRequest) -> Future<Result<AgentResponse, AgentError>>;
    fn name(&self) -> &str;
    fn is_available(&self) -> Future<bool>;
}

// Proveedores:
- MockProvider: Para tests, con latencia y respuestas configurables
- ClaudeProvider: Placeholder para API real (feature: claude-api)
```

### Archivos creados:
- `src/agent/mod.rs`
- `src/agent/request.rs`
- `src/agent/response.rs`
- `src/agent/bridge.rs`

### Tests: 8 tests unitarios pasando

---

## Track B: Capacidades Reales (50% completado)

**Objetivo:** Implementar capacidades que realmente funcionen.

### B1. +http ✓ COMPLETADO
```rust
// src/caps/http.rs - IMPLEMENTADO
pub fn http_get(url: &str, headers: Option<&HashMap<String, String>>) -> Result<Value, RuntimeError>
pub fn http_post(url: &str, body: Option<&str>, headers: Option<&HashMap<String, String>>) -> Result<Value, RuntimeError>
pub fn http_put(url: &str, body: Option<&str>, headers: Option<&HashMap<String, String>>) -> Result<Value, RuntimeError>
pub fn http_delete(url: &str, headers: Option<&HashMap<String, String>>) -> Result<Value, RuntimeError>
```

Retorna `Value::Record` con:
- `status: Int` - Código HTTP
- `headers: Record` - Headers de respuesta
- `body: String` - Cuerpo de respuesta

### Tests: 2 tests contra httpbin.org pasando

### B2. +json (pendiente)
```rust
// src/caps/json.rs - POR IMPLEMENTAR
pub fn json_parse(text: &str) -> Result<Value, RuntimeError>
pub fn json_stringify(value: &Value) -> Result<String, RuntimeError>
```

### B3. +db (futuro)
```rust
// src/caps/db.rs - FUTURO
// Usando SQLx para async DB
```

### Dependencias actuales:
```toml
reqwest = { version = "0.12", default-features = false, features = ["blocking", "json", "rustls-tls"] }
tokio = { version = "1.0", features = ["full"] }
```

---

## Track C: Hot Reload ✓ COMPLETADO

**Objetivo:** Permitir agregar código sin reiniciar el runtime.

### C1. Modo +append ✓ IMPLEMENTADO
```ruby
+append  # Habilita modo expansión

# Nuevo código se agrega al runtime existente
@NewType { ... }
new_function(x) = ...
```

### C2. Aplicar Diffs (por implementar)
- Parsear nuevo código
- Validar tipos
- Incorporar al environment
- Sin perder estado existente

### C3. Preservar Estado (por implementar)
- Variables globales persisten
- Conexiones DB persisten
- Solo se agregan/actualizan definiciones

### Archivos a crear:
```
src/reload/
├── mod.rs
├── diff.rs
└── apply.rs
```

---

## Track D: Developer Experience (50% completado)

**Objetivo:** Mejorar la experiencia de desarrollo.

### D1. REPL Mejorado ✓ COMPLETADO
- ✓ Evaluación real de expresiones
- ✓ VM persistente entre líneas
- ✓ Definición de funciones en sesión
- ✓ Comando :reset para reiniciar
- ✓ Comandos ?funcs, ?vars, ?help
- ○ Historial (pendiente)
- ○ Autocompletado básico (pendiente)
- ○ Colores (pendiente)

```
AURA REPL v0.1.0
> 2 + 3
5
> double(x) = x * 2
<fn double>
> double(21)
42
> ?funcs
Funciones definidas: double
```

### Funciones añadidas al parser:
```rust
pub fn parse_expression(tokens) -> Result<Expr, ParseError>
pub fn parse_function_def(tokens) -> Result<FuncDef, ParseError>
pub fn looks_like_function_def(tokens) -> bool
```

### Métodos añadidos a la VM:
```rust
impl VM {
    pub fn define_function(&mut self, func: FuncDef)
    pub fn list_functions(&self) -> Vec<String>
    pub fn list_variables(&self) -> Vec<String>
    pub fn reset(&mut self)
}
```

### D2. Errores Bonitos (pendiente)
```
Error en main.aura:15:8

  14 │   users = fetch_users!()
  15 │   report = generate_report(users)
     │            ^^^^^^^^^^^^^^^ Función no definida
  16 │   send_email!(report)

Sugerencia: Definir generate_report(users) = ...
```

### D3. LSP Básico (futuro)
- Hover con tipos
- Go to definition
- Errores en tiempo real

---

## Resumen de Progreso

| Track | Componente | Estado |
|-------|------------|--------|
| A | Request/Response | ✓ Completado |
| A | MockProvider | ✓ Completado |
| A | ClaudeProvider | ✓ Placeholder |
| A | Self-Healing | ✓ Completado |
| B | +http | ✓ Completado |
| B | +json | ✓ Completado |
| B | +db | ○ Futuro |
| C | +append | ✓ Completado |
| C | Diff/Apply | ✓ Completado |
| C | State Preserve | ✓ Completado |
| D | REPL básico | ✓ Completado |
| D | Errores bonitos | ○ Futuro |
| D | LSP | ○ Futuro |

## Métricas de Éxito

- [x] Agent puede recibir error y devolver fix
- [x] http.get!() funciona con URL real
- [x] Código nuevo se puede agregar sin reiniciar
- [ ] Errores muestran contexto de código (futuro)
- [x] REPL evalúa expresiones completas

## Tests

- **83 tests pasando** en total
- Agent Bridge: 8 tests
- Self-Healing: 13 tests
- HTTP Capability: 2 tests
- JSON Capability: 7 tests
- Hot Reload: 16 tests
- Lexer: 10 tests
- Parser: 5 tests
- Types: 5 tests
- VM: 5 tests

## Archivos Creados en Fase 2

```
src/
├── agent/
│   ├── mod.rs        # Exports del módulo
│   ├── request.rs    # AgentRequest, EventType, Context
│   ├── response.rs   # AgentResponse, Action, Patch
│   ├── bridge.rs     # AgentProvider trait, MockProvider
│   └── healing.rs    # HealingEngine, auto-reparación
├── caps/
│   ├── mod.rs        # Exports
│   ├── http.rs       # http_get, http_post, etc.
│   └── json.rs       # json_parse, json_stringify
└── reload/
    ├── mod.rs        # Exports e integration tests
    ├── diff.rs       # compute_diff, CodeDiff
    └── apply.rs      # apply_diff, hot_reload
```

## Próximos Pasos (Fase 3)

1. **Claude API real** - Implementar ClaudeProvider con API de Anthropic
2. **+db capability** - Base de datos con SQLx
3. **Errores bonitos** - Usar ariadne para mensajes de error
4. **LSP básico** - Language Server Protocol
