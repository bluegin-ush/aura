# v7 - Roadmap Fase 3: Self-Healing Seguro + Integración Real

**Fecha:** 2026-02-04
**Estado:** En progreso

## Objetivos

La Fase 3 se enfoca en dos áreas críticas:
1. **Seguridad del Self-Healing** - Sistema de snapshots/undo para revertar cambios fallidos
2. **Integración Real** - Claude API, +db, errores bonitos

```
┌─────────────────────────────────────────────────────────────────┐
│                     FASE 3: INTEGRACIÓN                         │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Track A          Track B          Track C          Track D     │
│  ─────────        ─────────        ─────────        ─────────   │
│  Safe Healing     Claude API       +db              DevEx       │
│                                                                 │
│  ┌─────────┐      ┌─────────┐      ┌─────────┐      ┌────────┐ │
│  │Snapshot │      │ API     │      │ SQLite  │      │ Errors │ │
│  │ System  │      │ Client  │      │ Driver  │      │ Bonitos│ │
│  └────┬────┘      └────┬────┘      └────┬────┘      └───┬────┘ │
│       │                │                │               │       │
│  ┌────▼────┐      ┌────▼────┐      ┌────▼────┐      ┌───▼────┐ │
│  │ Undo    │      │ Stream  │      │ Postgres│      │ LSP    │ │
│  │ Manager │      │ Support │      │ Driver  │      │ básico │ │
│  └────┬────┘      └────┬────┘      └────┬────┘      └───┬────┘ │
│       │                │                │               │       │
│  ┌────▼────┐      ┌────▼────┐      ┌────▼────┐      ┌───▼────┐ │
│  │ Auto    │      │ Claude  │      │ Query   │      │ Editor │ │
│  │ Revert  │      │Provider │      │ Builder │      │ Plugin │ │
│  └─────────┘      └─────────┘      └─────────┘      └────────┘ │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## Track A: Self-Healing Seguro ✓ COMPLETADO

**Problema:** El HealingEngine actual puede aplicar fixes automáticos, pero si el fix es incorrecto:
- El código queda roto
- No hay forma de revertir
- El agente podría entrar en un loop de errores

**Solución:** Sistema de Snapshots + Undo Manager

### A1. Snapshot System

```rust
// src/agent/snapshot.rs
pub struct Snapshot {
    pub id: SnapshotId,
    pub timestamp: DateTime<Utc>,
    pub source_hash: String,
    pub files: HashMap<PathBuf, FileSnapshot>,
    pub vm_state: Option<VMSnapshot>,
    pub reason: SnapshotReason,
}

pub struct FileSnapshot {
    pub path: PathBuf,
    pub content: String,
    pub hash: String,
}

pub enum SnapshotReason {
    BeforeHeal { error_id: String },
    BeforeHotReload,
    Manual { description: String },
    Checkpoint { interval: Duration },
}

pub struct SnapshotManager {
    snapshots: VecDeque<Snapshot>,
    max_snapshots: usize,
    storage_path: PathBuf,
}

impl SnapshotManager {
    pub fn create_snapshot(&mut self, reason: SnapshotReason) -> Result<SnapshotId, SnapshotError>
    pub fn restore_snapshot(&self, id: SnapshotId) -> Result<RestoreResult, SnapshotError>
    pub fn list_snapshots(&self) -> Vec<SnapshotSummary>
    pub fn prune_old_snapshots(&mut self) -> usize
}
```

### A2. Undo Manager

```rust
// src/agent/undo.rs
pub struct UndoManager {
    snapshot_manager: SnapshotManager,
    history: Vec<HealingAction>,
    current_position: usize,
}

pub struct HealingAction {
    pub snapshot_before: SnapshotId,
    pub patch_applied: Patch,
    pub timestamp: DateTime<Utc>,
    pub confidence: f32,
    pub verified: Option<VerificationResult>,
}

pub enum VerificationResult {
    Success { tests_passed: usize },
    Failure { error: String, tests_failed: usize },
    Timeout,
}

impl UndoManager {
    pub fn can_undo(&self) -> bool
    pub fn undo(&mut self) -> Result<UndoResult, UndoError>
    pub fn redo(&mut self) -> Result<RedoResult, UndoError>
    pub fn get_history(&self) -> &[HealingAction]
}
```

### A3. Safe Healing Pipeline

```rust
// src/agent/healing.rs - EXTENDER
impl HealingEngine {
    pub async fn heal_error_safe(
        &mut self,
        error: &RuntimeError,
        context: &HealingContext,
        undo_manager: &mut UndoManager,
    ) -> Result<SafeHealingResult, HealingError> {
        // 1. Crear snapshot antes de cualquier cambio
        let snapshot_id = undo_manager.create_snapshot(
            SnapshotReason::BeforeHeal { error_id: error.id.clone() }
        )?;

        // 2. Intentar healing normal
        let result = self.heal_error(error, context).await?;

        // 3. Si se aplicó fix, verificar
        if result.is_fixed() {
            let verification = self.verify_fix(&result, context).await;

            match verification {
                VerificationResult::Failure { .. } => {
                    // Auto-revert si la verificación falla
                    undo_manager.restore_snapshot(snapshot_id)?;
                    return Ok(SafeHealingResult::RevertedDueToFailure {
                        original_fix: result,
                        verification,
                    });
                }
                _ => {
                    // Registrar acción exitosa
                    undo_manager.record_action(HealingAction {
                        snapshot_before: snapshot_id,
                        patch_applied: result.get_patch().unwrap().clone(),
                        timestamp: Utc::now(),
                        confidence: result.confidence,
                        verified: Some(verification),
                    });
                }
            }
        }

        Ok(SafeHealingResult::Success(result))
    }

    async fn verify_fix(&self, result: &HealingResult, context: &HealingContext) -> VerificationResult {
        // 1. Re-parsear el código modificado
        // 2. Type check
        // 3. Ejecutar tests relacionados si existen
        // 4. Verificar que el error original no se repite
    }
}
```

### A4. CLI Commands para Undo

```bash
aura undo              # Revertir último fix
aura undo --list       # Listar historial de fixes
aura undo --to <id>    # Revertir a snapshot específico
aura snapshots         # Listar snapshots
aura snapshot create   # Crear snapshot manual
```

### Archivos a crear:
```
src/agent/
├── snapshot.rs    # Snapshot, SnapshotManager
├── undo.rs        # UndoManager, HealingAction
└── healing.rs     # Extender con heal_error_safe
```

---

## Track B: Claude API Real ✓ COMPLETADO

**Objetivo:** Implementar ClaudeProvider real usando la API de Anthropic.

**Estado:** Implementado en `src/agent/claude.rs`

### B1. API Client

```rust
// src/agent/claude.rs
#[cfg(feature = "claude-api")]
pub struct ClaudeProvider {
    api_key: String,
    model: String,
    base_url: String,
    http_client: reqwest::Client,
}

impl ClaudeProvider {
    pub fn new(api_key: String) -> Self
    pub fn with_model(self, model: &str) -> Self

    async fn call_api(&self, messages: Vec<Message>) -> Result<ApiResponse, ApiError>
}

impl AgentProvider for ClaudeProvider {
    async fn send_request(&self, req: AgentRequest) -> Result<AgentResponse, AgentError> {
        // 1. Convertir AgentRequest a prompt de Claude
        let prompt = self.format_healing_prompt(&req);

        // 2. Llamar API
        let response = self.call_api(vec![
            Message::system(AURA_HEALING_SYSTEM_PROMPT),
            Message::user(prompt),
        ]).await?;

        // 3. Parsear respuesta a AgentResponse
        self.parse_healing_response(&response)
    }
}
```

### B2. System Prompt para Healing

```rust
const AURA_HEALING_SYSTEM_PROMPT: &str = r#"
Eres un agente de reparación para AURA, un lenguaje de programación.

Cuando recibes un error, debes:
1. Analizar el contexto del código
2. Identificar la causa raíz
3. Proponer un fix con alta confianza

Responde SIEMPRE en JSON con este formato:
{
    "action": "patch" | "suggest" | "escalate",
    "patch": {
        "file": "path/to/file.aura",
        "line": 10,
        "old_text": "código original",
        "new_text": "código corregido"
    },
    "explanation": "Por qué este fix funciona",
    "confidence": 0.0-1.0
}

Sintaxis AURA:
- Capacidades: +http +json +db
- Tipos: @User { name:s age:i }
- Funciones: fn(args) = expr
- Efectos: fn!() indica side effects
- Pipes: x | filter(...) | map(...)
"#;
```

### B3. Rate Limiting & Retry

```rust
pub struct RateLimiter {
    requests_per_minute: u32,
    last_requests: VecDeque<Instant>,
}

impl ClaudeProvider {
    async fn call_with_retry(&self, messages: Vec<Message>) -> Result<ApiResponse, ApiError> {
        let mut attempts = 0;
        let max_attempts = 3;

        loop {
            self.rate_limiter.wait_if_needed().await;

            match self.call_api(messages.clone()).await {
                Ok(response) => return Ok(response),
                Err(ApiError::RateLimit) if attempts < max_attempts => {
                    attempts += 1;
                    tokio::time::sleep(Duration::from_secs(2u64.pow(attempts))).await;
                }
                Err(e) => return Err(e),
            }
        }
    }
}
```

### Dependencias a agregar:
```toml
[dependencies]
chrono = { version = "0.4", features = ["serde"] }

[features]
claude-api = ["dep:reqwest"]
```

---

## Track C: +db Capability

**Objetivo:** Acceso a bases de datos SQL.

### C1. SQLite Driver

```rust
// src/caps/db.rs
pub struct DbConnection {
    pool: Pool<Sqlite>,  // o Pool<Postgres>
}

pub fn db_connect(url: &str) -> Result<Value, RuntimeError>
pub fn db_query(conn: &Value, sql: &str, params: &[Value]) -> Result<Value, RuntimeError>
pub fn db_execute(conn: &Value, sql: &str, params: &[Value]) -> Result<Value, RuntimeError>
pub fn db_close(conn: &Value) -> Result<(), RuntimeError>
```

### C2. Query Builder (Futuro)

```rust
// Sintaxis AURA para queries type-safe:
// users | where(_.active) | select(_.name, _.email) | limit(10)

pub fn query_to_sql(query: &QueryExpr, schema: &Schema) -> String
```

### Uso en AURA:
```ruby
+db

@User { id:uuid @pk name:s email:s }

main = {
    conn = db.connect!("sqlite:app.db")

    # Query simple
    users = db.query!(conn, "SELECT * FROM users WHERE active = ?", [true])

    # O con builder (futuro)
    users = User | where(_.active) | limit(10)

    users | map(_.name) | each(print!)
}
```

### Dependencias:
```toml
sqlx = { version = "0.7", features = ["runtime-tokio", "sqlite", "postgres"] }
```

---

## Track D: Developer Experience

### D1. Errores Bonitos

```rust
// src/error/pretty.rs
use ariadne::{Report, ReportKind, Source, Label};

pub fn format_error_pretty(error: &AuraError, source: &str) -> String {
    Report::build(ReportKind::Error, error.file(), error.span())
        .with_message(&error.message())
        .with_label(
            Label::new((error.file(), error.span()))
                .with_message(&error.hint())
                .with_color(Color::Red)
        )
        .finish()
        .print((error.file(), Source::from(source)))
}
```

Output:
```
Error: Variable no definida
   ╭─[main.aura:15:8]
   │
14 │   users = fetch_users!()
15 │   report = generate_report(users)
   │            ^^^^^^^^^^^^^^^ Esta función no existe
16 │   send_email!(report)
   │
   ├─ Sugerencia: Definir generate_report(users) = ...
   ╰─
```

### D2. LSP Básico (Futuro)

- Hover con tipos
- Go to definition
- Errores en tiempo real
- Autocompletado

---

## Métricas de Éxito

- [ ] Snapshot se crea antes de cada auto-fix
- [ ] `aura undo` revierte el último fix
- [ ] Fix fallido se auto-revierte
- [ ] Claude API funciona con healing real
- [ ] +db conecta a SQLite
- [ ] Errores muestran código con colores

## Orden de Implementación

1. **Track A: Snapshots** (crítico para seguridad)
   - A1: SnapshotManager básico
   - A2: UndoManager
   - A3: heal_error_safe
   - A4: CLI commands

2. **Track B: Claude API** (depende de A para seguridad)
   - B1: ClaudeProvider
   - B2: System prompt
   - B3: Rate limiting

3. **Track D: Errores bonitos** (mejora feedback)
   - D1: format_error_pretty

4. **Track C: +db** (puede ser paralelo)
   - C1: SQLite driver

## Tests Esperados

- Snapshot: crear, restaurar, prune
- Undo: undo/redo básico, límite de historial
- Safe Healing: auto-revert on failure
- Claude API: mock integration, rate limiting
- +db: connect, query, execute
