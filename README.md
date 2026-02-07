# AURA

## Un lenguaje de programación diseñado para agentes IA

> **Los lenguajes de programación fueron diseñados para humanos.**
> **AURA fue diseñado para máquinas que escriben código.**

---

## Tres Virtudes

### 1. 40x Menos Tokens

```
    Python   ████████████████████████████ 2000 tokens
    AURA     ██ 50 tokens
```

Un agente IA consume 40 veces menos tokens escribiendo AURA que Python.
Menos tokens = menos costo = más operaciones por dólar.

### 2. Self-Healing: Se Repara Solo

```bash
$ aura heal broken.aura

═══════════════════════════════════════════════════════════════
   AURA Self-Healing Demo
═══════════════════════════════════════════════════════════════

📄 File: broken.aura
🔧 Provider: claude

1️⃣ Original code:

   1  double(n) = n * 2
   2  main = double(x)        # ← Error: 'x' no definida

2️⃣ Attempting to execute...
❌ Runtime error detected: Variable no definida: x

3️⃣ Initiating self-healing...
4️⃣ Consulting claude agent...

🔍 Agent analysis:
   La variable 'x' no está definida. Se debe declarar antes de usar.

5️⃣ Proposed fix:

   --- Original
   +++ Fixed

   - main = double(x)
   + x = 21
   + main = double(x)

6️⃣ Applying fix...
7️⃣ Re-executing...

🎉 SUCCESS! Fixed code executes correctly!
   Result: 42

═══════════════════════════════════════════════════════════════
   Self-Healing Complete!
═══════════════════════════════════════════════════════════════
```

El código se detecta, analiza y repara automáticamente. Sin intervención humana.

### 3. Un Archivo = Todo

```
    Python/Flask  ████████████████████████████  10 archivos
    AURA          ██  1 archivo
```

No hay `requirements.txt`, `config.py`, `models.py`, `routes.py`...
Todo el contexto en un solo lugar. El agente no pierde tiempo navegando.

---

## Probalo Ahora

```bash
# Instalar
git clone https://github.com/bluegin-ush/aura && cd aura
cargo build --release

# Ejecutar programa
./target/release/aura run examples/01_api_client.aura

# Demo de self-healing
./target/release/aura heal examples/broken.aura

# Iniciar API REST
./target/release/aura serve api.aura --port 8080

# REPL interactivo
./target/release/aura repl
```

---

## Ejemplos Reales

### API Client (4 líneas)
```ruby
+http +json

get_user(id) = : r = http.get("https://api.com/users/{id}"); json.parse(r.body)
main = : user = get_user(1); "User: {user.name}"
```

### CRUD Database (8 líneas)
```ruby
+db

conn = db.connect("sqlite:./app.db")
get_users = db.query(conn(), "SELECT * FROM users", [])
create_user(name email) = db.execute(conn(), "INSERT INTO users (name, email) VALUES (?, ?)", [name, email])
main = : create_user("Alice", "alice@test.com"); get_users()
```

### REST API Server (10 líneas)
```ruby
+db +json

conn = db.connect("sqlite:./api.db")

get_health = {status: "ok"}
get_users = db.query(conn(), "SELECT * FROM users", [])
get_user(id) = first(db.query(conn(), "SELECT * FROM users WHERE id = ?", [id]))
post_user(req) = : db.execute(conn(), "INSERT INTO users (name) VALUES (?)", [req.body.name]); {created: true}
del_user(id) = : db.execute(conn(), "DELETE FROM users WHERE id = ?", [id]); {deleted: true}
```

```bash
$ aura serve api.aura --port 8080
Routes:
  GET /health
  GET /users
  GET /user/:id
  POST /user
  DELETE /user/:id
```

---

## Caso de Estudio: MotoStock

Sistema completo de gestión de inventario para taller de motos.
**23 endpoints REST** en **68 líneas** de código.
Desarrollado por un agente IA en **35 minutos**.

```
                         AURA vs Python/Flask
    ┌────────────────────────────────────────────────────────┐
    │                                                        │
    │  Líneas de código    68  vs  450     (85% menos)      │
    │  Archivos             1  vs   10     (90% menos)      │
    │  Tokens consumidos   3K  vs  15K     (80% menos)      │
    │  Tiempo             35m  vs   4h     (85% menos)      │
    │                                                        │
    └────────────────────────────────────────────────────────┘
```

![Dashboard](projects/motostock/screenshots/dashboard.png)

**[→ Ver proyecto completo](projects/motostock/)**

---

## Sintaxis

```ruby
# Capacidades (reemplazan imports)
+http +json +db

# Todo es una función
x = 42                    # x() retorna 42
double(n) = n * 2         # función con parámetro

# Bloques con valores intermedios
process(x) = : a = x * 2; b = a + 10; b

# Pipes funcionales
result = data |> transform |> filter |> save

# Condicionales expresivos
abs(n) = if n < 0 (-n) else n

# Interpolación
msg = "Hola {user.name}, tienes {count} mensajes"

# Records
user = {name: "Alice", age: 30}

# Listas
nums = [1, 2, 3, 4, 5]
```

---

## Self-Healing: Cómo Funciona

```
   ┌─────────────┐     Error      ┌─────────────┐
   │   Runtime   │ ──────────────▶│   Agente    │
   │    AURA     │                │  (Claude)   │
   │             │ ◀──────────────│             │
   └─────────────┘      Fix       └─────────────┘
         │                              │
         ▼                              ▼
   ┌─────────────┐               ┌─────────────┐
   │  Snapshot   │               │   Analiza   │
   │  (backup)   │               │   contexto  │
   └─────────────┘               └─────────────┘
         │                              │
         ▼                              ▼
   ┌─────────────┐               ┌─────────────┐
   │  Aplica     │◀──────────────│  Genera     │
   │  patch      │    Patch      │  solución   │
   └─────────────┘               └─────────────┘
         │
         ▼
   ┌─────────────┐
   │  Verifica   │──▶ Si falla, revierte al snapshot
   │  ejecución  │
   └─────────────┘
```

**Proveedores soportados:**
- Claude (Anthropic API)
- OpenAI
- Ollama (local)

```bash
# Demo con mock (sin API key)
aura heal broken.aura

# Con Claude
ANTHROPIC_API_KEY=sk-xxx aura heal broken.aura --provider claude

# Con Ollama local
aura heal broken.aura --provider ollama
```

---

## Comandos

| Comando | Descripción |
|---------|-------------|
| `aura run file.aura` | Ejecutar programa |
| `aura heal file.aura` | Demo de self-healing |
| `aura serve file.aura` | Iniciar servidor HTTP |
| `aura repl` | REPL interactivo |
| `aura check file.aura` | Verificar sin ejecutar |
| `aura undo` | Revertir último fix |
| `aura snapshots` | Gestionar snapshots |

---

## Capacidades

| Capacidad | Funciones |
|-----------|-----------|
| `+http` | `http.get`, `http.post`, `http.put`, `http.delete` |
| `+json` | `json.parse`, `json.stringify` |
| `+db` | `db.connect`, `db.query`, `db.execute` |
| `+math` | `sqrt`, `pow`, `sin`, `cos`, `log` |
| `+time` | `time.now`, `time.format`, `time.parse` |
| `+crypto` | `crypto.hash`, `crypto.hmac` |

---

## Estado

```
✅ Intérprete completo
✅ REPL interactivo
✅ Servidor HTTP nativo
✅ Self-healing con Claude/OpenAI/Ollama
✅ Sistema de snapshots y undo
✅ 62 tests pasando
```

---

## Parte de Algo Más Grande

| Proyecto | Pregunta |
|----------|----------|
| [**Y@ enseño {con IA}**](https://github.com/bluegin-ush/yo-ense-o-con-IA-) | ¿Qué debe saber un profesional en la era de la IA? |
| **AURA** (estás acá) | Si la IA escribe código, ¿con qué lenguaje? |
| [**IS-IA**](https://github.com/bluegin-ush/IS-IA) | ¿Cómo se construye software con IA? |

---

## Licencia

MIT

---

<p align="center">
<strong>AURA: Porque el futuro del código lo escriben máquinas.</strong>
</p>
