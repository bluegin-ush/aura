# v4 - Protocolo Agente-Runtime

## Visión

El runtime de AURA puede comunicarse con agentes de IA para:
1. **Self-healing**: Auto-reparar errores
2. **Expansión**: Generar código faltante bajo demanda
3. **Optimización**: Mejorar código basado en métricas
4. **Debugging**: Asistir en resolución de problemas

## Arquitectura del Bridge

```
┌─────────────────────────────────────────────────────────────────┐
│                        AURA RUNTIME                             │
│                                                                 │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │                     Agent Bridge                         │   │
│  │                                                          │   │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐              │   │
│  │  │  Event   │  │ Context  │  │ Response │              │   │
│  │  │ Detector │  │ Packager │  │ Applier  │              │   │
│  │  └────┬─────┘  └────┬─────┘  └────┬─────┘              │   │
│  │       │             │             │                     │   │
│  │       └─────────────┼─────────────┘                     │   │
│  │                     │                                    │   │
│  └─────────────────────┼────────────────────────────────────┘   │
│                        │                                        │
└────────────────────────┼────────────────────────────────────────┘
                         │
                         ▼
              ┌─────────────────────┐
              │   Agent Provider    │
              │                     │
              │  ┌───────────────┐  │
              │  │   Anthropic   │  │
              │  │   (Claude)    │  │
              │  └───────────────┘  │
              │  ┌───────────────┐  │
              │  │    OpenAI     │  │
              │  └───────────────┘  │
              │  ┌───────────────┐  │
              │  │  Local LLM    │  │
              │  │  (Ollama)     │  │
              │  └───────────────┘  │
              └─────────────────────┘
```

## Configuración

```ruby
+agent(
  provider: "anthropic"              # "anthropic" | "openai" | "ollama" | "custom"
  model: "claude-sonnet-4"           # Modelo específico
  api_key: @env.AGENT_API_KEY        # Desde variable de entorno

  # Comportamientos
  on_error: .fix                     # .fix | .ask | .log | .fail
  on_missing: .generate              # .generate | .fail
  on_slow: .optimize                 # .optimize | .ignore
  on_test_fail: .fix                 # .fix | .log | .fail

  # Controles
  confirm: false                     # Pedir confirmación antes de aplicar
  max_retries: 3                     # Intentos máximos por problema
  timeout: 30.seconds                # Timeout por request
  budget: {
    daily_tokens: 100000             # Límite diario
    per_request: 4000                # Límite por request
  }

  # Contexto
  include_source: true               # Enviar código fuente relevante
  include_types: true                # Enviar definiciones de tipos
  include_history: 5                 # Últimos N errores/cambios
)
```

## Protocolo de comunicación

### Request al agente

```json
{
  "version": "1.0",
  "request_id": "uuid",
  "timestamp": "2026-02-04T10:30:00Z",

  "event": {
    "type": "error",
    "code": "E101",
    "message": "Tipo incompatible: esperado User, recibido {data: User}"
  },

  "location": {
    "file": "main.aura",
    "line": 15,
    "col": 8,
    "function": "fetch_user"
  },

  "context": {
    "source": "fetch_user(id) = http.get!(\"users/{id}\").json(User)",
    "types": {
      "User": {"id": "uuid", "name": "s", "email": "s?"}
    },
    "surrounding_code": [
      {"line": 14, "code": "# Obtiene usuario por ID"},
      {"line": 15, "code": "fetch_user(id) = http.get!(\"users/{id}\").json(User)"},
      {"line": 16, "code": ""}
    ],
    "runtime_state": {
      "input": {"id": 123},
      "http_response": {"data": {"id": 123, "name": "John"}}
    }
  },

  "history": [
    {"timestamp": "...", "event": "...", "resolution": "..."}
  ],

  "constraints": {
    "max_tokens": 4000,
    "response_format": "aura_patch"
  }
}
```

### Response del agente

```json
{
  "version": "1.0",
  "request_id": "uuid",

  "action": "patch",

  "patch": {
    "type": "replace",
    "file": "main.aura",
    "line": 15,
    "old": "fetch_user(id) = http.get!(\"users/{id}\").json(User)",
    "new": "fetch_user(id) = http.get!(\"users/{id}\").json().data.as(User)"
  },

  "explanation": "La API devuelve {data: User}, necesitamos extraer .data antes de convertir a User",

  "confidence": 0.95,

  "tests_suggested": [
    "#test fetch_user_ok: fetch_user(1).name == \"John\""
  ]
}
```

## Tipos de eventos

### 1. Error de runtime

```json
{
  "type": "error",
  "subtype": "type_mismatch",
  "code": "E101",
  "message": "..."
}
```

**Acciones posibles:** `patch`, `suggest`, `explain`, `escalate`

### 2. Función no encontrada

```json
{
  "type": "missing",
  "subtype": "function",
  "name": "generate_report",
  "called_from": "main!",
  "expected_signature": {
    "args": [{"name": "users", "type": "[User]"}],
    "return": "s"
  }
}
```

**Acciones posibles:** `generate`, `suggest_import`, `escalate`

### 3. Performance degradada

```json
{
  "type": "performance",
  "subtype": "slow_query",
  "function": "list_users",
  "metrics": {
    "avg_time_ms": 2500,
    "p99_time_ms": 8000,
    "calls_per_minute": 100
  },
  "profile": {
    "db_time_ms": 2300,
    "serialization_ms": 200
  }
}
```

**Acciones posibles:** `optimize`, `suggest_index`, `cache`, `ignore`

### 4. Test fallido

```json
{
  "type": "test_failure",
  "test": "user_email_valid",
  "assertion": "validate_email(\"test@\") == false",
  "result": "true",
  "expected": "false"
}
```

**Acciones posibles:** `fix_code`, `fix_test`, `explain`, `escalate`

### 5. Expansión solicitada

```json
{
  "type": "expansion",
  "request": "Agregar endpoint para eliminar posts en batch",
  "context": {
    "existing_endpoints": ["GET /posts", "POST /posts", "..."],
    "related_types": ["Post", "User"]
  }
}
```

**Acciones posibles:** `generate`, `clarify`, `refuse`

## Tipos de acciones

### patch
```json
{
  "action": "patch",
  "patch": {
    "type": "replace" | "insert" | "delete",
    "file": "...",
    "line": 15,
    "old": "...",                   // Para replace/delete
    "new": "..."                    // Para replace/insert
  }
}
```

### generate
```json
{
  "action": "generate",
  "code": "new_function(x) = x * 2",
  "location": {
    "file": "main.aura",
    "after_line": 20
  }
}
```

### suggest
```json
{
  "action": "suggest",
  "suggestions": [
    {
      "description": "Extraer .data de la respuesta",
      "patch": {...},
      "confidence": 0.9
    },
    {
      "description": "Cambiar tipo de respuesta esperado",
      "patch": {...},
      "confidence": 0.7
    }
  ]
}
```

### clarify
```json
{
  "action": "clarify",
  "questions": [
    "¿La API siempre devuelve {data: ...} o solo en algunos casos?",
    "¿Debería manejar el caso donde data es null?"
  ]
}
```

### escalate
```json
{
  "action": "escalate",
  "reason": "El problema requiere cambios arquitectónicos",
  "analysis": "...",
  "recommended_changes": [...]
}
```

## Flujos de interacción

### Self-healing automático

```
1. Runtime detecta error
2. Event Detector captura contexto
3. Context Packager arma request
4. Request enviado al agente
5. Agente analiza y responde con patch
6. Response Applier valida patch
7. Si válido: aplica hot reload
8. Si inválido: retry o escalate
9. Continúa ejecución
```

### Expansión bajo demanda

```
1. Código llama función inexistente
2. Runtime pausa ejecución
3. Request enviado al agente con contexto
4. Agente genera la función
5. Runtime valida tipos
6. Hot reload con nueva función
7. Continúa ejecución desde punto de pausa
```

### Optimización continua

```
1. Runtime recolecta métricas
2. Detecta patrones problemáticos
3. Request enviado al agente con métricas
4. Agente sugiere optimización
5. Si confirm:false, aplica automáticamente
6. Si confirm:true, notifica y espera
```

## Seguridad

### Sandbox para código generado

```ruby
+agent(
  sandbox: {
    allow: [+http +json]             # Capacidades permitidas
    deny: [+fs +crypto]              # Capacidades prohibidas
    max_memory: 100.mb
    max_cpu: 5.seconds
    network: {
      allow: ["api.internal.com"]
      deny: ["*"]
    }
  }
)
```

### Validación de patches

Antes de aplicar cualquier patch:
1. Parse y type check del nuevo código
2. Verificar que no rompe tipos existentes
3. Ejecutar tests afectados
4. Verificar constraints de seguridad

### Audit log

```json
{
  "timestamp": "2026-02-04T10:30:00Z",
  "event_type": "patch_applied",
  "request_id": "uuid",
  "agent": "claude-sonnet-4",
  "patch": {...},
  "validation": {
    "type_check": "pass",
    "tests": "pass",
    "security": "pass"
  },
  "applied_by": "auto" | "user_confirmed"
}
```

## CLI para interacción manual

```bash
# Ver estado del agente
aura agent status

# Pedir expansión manual
aura agent expand "Agregar validación de email"

# Ver historial de cambios del agente
aura agent history

# Revertir último cambio del agente
aura agent revert

# Pausar agente
aura agent pause

# Configurar interactivamente
aura agent config
```

## Métricas

El runtime trackea:

```ruby
agent.metrics = {
  requests_total: i
  requests_successful: i
  requests_failed: i

  patches_applied: i
  patches_rejected: i
  patches_reverted: i

  avg_response_time_ms: f
  tokens_used_today: i

  errors_auto_fixed: i
  errors_escalated: i
}
```

## Próximo

Implementación en Rust comenzando por:
1. Lexer (tokens básicos)
2. Parser (AST)
3. Type checker
4. VM simple
5. Agent bridge
