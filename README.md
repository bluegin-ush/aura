# AURA

**Agent-Unified Runtime Architecture**

Un lenguaje de programación diseñado específicamente para agentes de IA.

## Por qué AURA

Los lenguajes actuales fueron diseñados para humanos. Cuando un agente de IA los usa, enfrenta:

- **Contexto fragmentado** - 6+ archivos para entender una función
- **Boilerplate repetitivo** - Imports en cada archivo
- **Errores inútiles** - Stack traces de 50 líneas
- **Múltiples formas** - Inconsistencia en el código generado

AURA resuelve esto con:

| Problema | Solución AURA |
|----------|---------------|
| Imports | Capacidades: `+http +json` |
| Tipos separados | Inline: `@User { name:s age:i }` |
| Errores crípticos | JSON estructurado con sugerencias |
| Config explosiva | Convención sobre configuración |
| Código roto | Self-healing con agentes IA |

## Instalación

```bash
git clone https://github.com/tu-usuario/aura
cd aura
cargo build --release
```

## Uso Rápido

```ruby
# hello.aura
+core

greeting(name) = "Hello {name}!"

main = greeting("AURA")
```

```bash
$ aura run hello.aura
Hello AURA!
```

## Sintaxis

### Capacidades
```ruby
+http +json +db +auth    # Habilita funcionalidades
```

### Tipos
```ruby
@User {
    id:uuid @pk
    name:s @min(2) @max(100)
    email:s? @email          # ? = nullable
    role:Role = .user        # Default value
}

@Role = admin | user | guest  # Enum
```

### Funciones
```ruby
# Función pura
add(a b) = a + b

# Función con efectos (IO)
fetch!(url) = http.get!(url).json()

# Sin parámetros
main = greeting("World")
```

### Expresiones
```ruby
# Pipes
users | filter(_.active) | map(_.name) | sort

# Pattern matching
handle(r) = r | Ok(v) -> v | Err(e) -> nil

# Null coalescing
name = user?.name ?? "Anonymous"

# Interpolación
msg = "Hello {user.name}, you have {count} messages"
```

## Comandos CLI

```bash
aura run <file>       # Ejecutar programa
aura check <file>     # Verificar tipos
aura parse <file>     # Ver AST (--json para JSON)
aura lex <file>       # Ver tokens
aura repl             # REPL interactivo
aura info             # Info del runtime (--json)
```

## Capacidades Implementadas

### +http
```rust
// Rust API
use aura::caps::{http_get, http_post, http_put, http_delete};

let response = http_get("https://api.example.com/users", None)?;
// Returns Value::Record { status, headers, body }
```

### +json
```rust
use aura::caps::{json_parse, json_stringify};

let value = json_parse(r#"{"name": "AURA", "version": 1}"#)?;
let text = json_stringify(&value)?;
```

### +db
```rust
use aura::caps::{db_connect, db_query, db_execute, db_close};

// Conectar a SQLite
let conn = db_connect("sqlite:app.db")?;  // o ":memory:" para in-memory

// Crear tabla
db_execute(&conn, "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT)", &[])?;

// Insertar
db_execute(&conn, "INSERT INTO users (name) VALUES (?)", &[Value::String("Alice".into())])?;

// Consultar
let users = db_query(&conn, "SELECT * FROM users", &[])?;

// Cerrar conexión
db_close(&conn)?;
```

## Agent Bridge

AURA puede comunicarse con agentes IA para:

- **Self-healing** - Auto-reparar errores en runtime
- **Expansión** - Generar código faltante bajo demanda
- **Optimización** - Mejorar código basado en métricas

### Protocolo

```rust
use aura::agent::{AgentRequest, AgentResponse, EventType, MockProvider};

// Crear request cuando hay un error
let request = AgentRequest::error(code, file, line, col)
    .with_message("Variable 'x' no definida");

// Enviar al agente
let response = provider.send_request(request).await?;

// Procesar respuesta
match response.action {
    Action::Patch => { /* aplicar fix */ }
    Action::Suggest => { /* mostrar sugerencias */ }
    Action::Escalate => { /* requiere humano */ }
}
```

### Proveedores de Agentes

AURA soporta múltiples proveedores de agentes IA:

#### Claude API
```rust
// cargo build --features claude-api
use aura::agent::{ClaudeProvider, HealingEngine};

let provider = ClaudeProvider::new("sk-ant-your-api-key")
    .with_model("claude-sonnet-4-20250514");

let mut engine = HealingEngine::new(provider)
    .with_auto_apply(true);
```

#### Ollama (Local)
```rust
// cargo build --features ollama
use aura::agent::{OllamaProvider, HealingEngine};

let provider = OllamaProvider::new()
    .with_model("llama3.2")
    .with_base_url("http://localhost:11434");

let mut engine = HealingEngine::new(provider)
    .with_auto_apply(true);

// Verificar disponibilidad
if provider.is_available().await {
    let result = engine.heal_error(&error, &context).await?;
}
```

### Self-Healing Engine

```rust
use aura::agent::{HealingEngine, HealingContext, MockProvider};

let provider = MockProvider::new();
let mut engine = HealingEngine::new(provider)
    .with_auto_apply(true)
    .with_confidence_threshold(0.8);

let error = RuntimeError::new("Variable no definida: x");
let context = HealingContext::new("x + 1", "main.aura", 1, 1);

let result = engine.heal_error(&error, &context).await?;
if result.is_fixed() {
    println!("Reparado: {:?}", result.get_patch());
}
```

### Safe Healing con Snapshots

El sistema de Safe Healing permite revertir fixes fallidos:

```rust
use aura::agent::{HealingEngine, HealingContext, MockProvider, UndoManager, SnapshotManager};

// Crear managers
let snapshot_manager = SnapshotManager::new(50); // Max 50 snapshots
let mut undo_manager = UndoManager::new(snapshot_manager);

let provider = MockProvider::new();
let mut engine = HealingEngine::new(provider)
    .with_auto_apply(true);

let error = RuntimeError::new("Variable no definida: x");
let context = HealingContext::new("x + 1", "main.aura", 1, 1);

// Healing seguro - crea snapshot antes de aplicar fix
let result = engine.heal_error_safe(&error, &context, &mut undo_manager, "x + 1").await?;

// Si el fix falla, se puede revertir
if undo_manager.can_undo() {
    let (action, snapshot) = undo_manager.prepare_undo()?;
    // Restaurar archivos desde el snapshot
    for (path, file_snap) in &snapshot.files {
        std::fs::write(path, &file_snap.content)?;
    }
    undo_manager.confirm_undo();
}
```

## Hot Reload

Agregar código sin reiniciar el runtime:

```rust
use aura::reload::{compute_diff, apply_diff, hot_reload};

// Detectar cambios
let diff = compute_diff(&program, "double(x) = x * 2")?;

// Aplicar sin perder estado
let result = apply_diff(&mut vm, diff)?;
println!("Funciones agregadas: {}", result.functions_added);

// O usar el atajo
let result = hot_reload(&mut vm, &program, new_code)?;
```

## Errores Bonitos

AURA formatea errores con contexto de código:

```
Error[E201]: 'generate_report' no esta definido
   --> main.aura:10:10
    |
 10 | report = generate_report(users)
    |          ^^^^^^^^^^^^^^^ referencia invalida
    |
    = help: Definir la funcion: generate_report(users) = ...
```

```rust
use aura::error::{format_error_pretty, AuraError};

let output = format_error_pretty(&error, source_code, "main.aura");
println!("{}", output);
```

## REPL Interactivo

```
$ aura repl
AURA REPL v0.1.0
Escribe 'exit' para salir, ':reset' para reiniciar

> 2 + 3
5
> double(x) = x * 2
<fn double>
> double(21)
42
> ?funcs
Funciones definidas: double
> :reset
Estado reiniciado
```

## Comparación

**Python típico (~400 líneas, 8 archivos, ~2000 tokens):**
```python
import requests
from typing import Optional
from dataclasses import dataclass
# ... setup, config, tipos, validación, etc.
```

**AURA equivalente (~4 líneas, 1 archivo, ~50 tokens):**
```ruby
+http +json
@User {id:uuid @pk name:s email:s?}
fetch(id) = http.get!("users/{id}").json(User)
```

**Reducción: 98% menos tokens**

## Arquitectura

```
src/
├── lexer/          # Tokenización con logos
├── parser/         # Parser recursivo descendente
├── types/          # Type checker básico
├── vm/             # Máquina virtual interpretada
├── agent/          # Agent Bridge + Self-Healing
│   ├── request.rs  # AgentRequest, EventType
│   ├── response.rs # AgentResponse, Action
│   ├── bridge.rs   # AgentProvider trait, MockProvider
│   ├── healing.rs  # HealingEngine
│   ├── snapshot.rs # Snapshots para safe healing
│   ├── undo.rs     # Undo manager
│   ├── claude.rs   # ClaudeProvider (feature: claude-api)
│   └── ollama.rs   # OllamaProvider (feature: ollama)
├── caps/           # Capacidades
│   ├── http.rs     # +http
│   ├── json.rs     # +json
│   └── db.rs       # +db (SQLite)
├── reload/         # Hot Reload
│   ├── diff.rs     # compute_diff
│   └── apply.rs    # apply_diff
└── error/          # Errores estructurados
    ├── mod.rs      # AuraError, ErrorCode
    └── pretty.rs   # Formateo con ariadne
```

## Estado del Proyecto

```
✓ Lexer        - Tokenización completa
✓ Parser       - AST completo
✓ Types        - Verificación básica
✓ VM           - Interpretación con interpolación
✓ Agent Bridge - Protocolo request/response
✓ Self-Healing - HealingEngine con auto-apply
✓ Safe Healing - Snapshots + Undo para revertir fixes fallidos
✓ +http        - GET, POST, PUT, DELETE
✓ +json        - parse, stringify
✓ Hot Reload   - compute_diff, apply_diff
✓ REPL         - VM persistente, comandos

✓ Claude API   - Integración con API de Anthropic
✓ Ollama       - Soporte para modelos locales
✓ +db          - SQLite con rusqlite
✓ Errores UI   - Formateo con ariadne
```

## Tests

```bash
cargo test
# 159 tests pasando (con --features claude-api,ollama)
```

## Documentación

### Para Agentes IA

**[AGENT_GUIDE.md](AGENT_GUIDE.md)** - Guía completa para que agentes IA comprendan AURA:
- Sintaxis completa y ejemplos
- Sistema de capacidades
- Catálogo de errores y fixes
- Protocolo de healing
- Patrones idiomáticos

### Especificaciones

Ver carpeta `req/` para especificaciones completas:

- `v1-vision-principios.md` - Por qué existe AURA
- `v2-sintaxis.md` - Gramática completa
- `v3-runtime.md` - Cómo funciona la VM
- `v4-agente-bridge.md` - Protocolo agente-runtime
- `v5-implementacion-fase1.md` - Fase 1 completada
- `v6-roadmap-fase2.md` - Fase 2 completada
- `v7-roadmap-fase3.md` - Fase 3 en progreso

## Contribuir

AURA está diseñado para que agentes de IA contribuyan. El código es:
- Mínimo y no ambiguo
- Con errores estructurados en JSON
- Auto-documentado con tipos

## Licencia

MIT
