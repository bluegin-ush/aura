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

```ruby
goal "calcular el doble de un número"

double(n) = n * 2
main = double(x)   # ← Error: 'x' no definida
```

```
$ aura heal broken.aura

❌ Error: Variable no definida: x
🔍 Goal: "calcular el doble de un número"
🤖 Analizando...
✅ Fix aplicado: x = 21

Resultado: 42
```

El código se detecta, analiza y repara automáticamente. El `goal` le dice al agente **qué querías lograr**, no solo qué falló.

**[→ Ver documentación completa de Self-Healing](SELF-HEALING.md)**

### 3. Un Archivo = Todo

```
    Python/Flask  ████████████████████████████  10 archivos
    AURA          ██  1 archivo
```

No hay `requirements.txt`, `config.py`, `models.py`, `routes.py`...
Todo el contexto en un solo lugar.

---

## Probalo Ahora

```bash
git clone https://github.com/bluegin-ush/aura && cd aura
cargo build --release

./target/release/aura run examples/01_api_client.aura    # Ejecutar
./target/release/aura heal examples/broken.aura          # Self-healing
./target/release/aura serve api.aura --port 8080         # API REST
./target/release/aura repl                               # REPL
```

---

## Ejemplo: API REST en 10 líneas

```ruby
+db +json

goal "API REST para gestión de usuarios"

conn = db.connect("sqlite:./api.db")

get_health = {status: "ok"}
get_users = db.query(conn(), "SELECT * FROM users", [])
get_user(id) = first(db.query(conn(), "SELECT * FROM users WHERE id = ?", [id]))
post_user(req) = : db.execute(conn(), "INSERT INTO users (name) VALUES (?)", [req.body.name]); {created: true}
del_user(id) = : db.execute(conn(), "DELETE FROM users WHERE id = ?", [id]); {deleted: true}
```

```bash
$ aura serve api.aura --port 8080
Routes: GET /health, GET /users, GET /user/:id, POST /user, DELETE /user/:id
```

---

## Caso de Estudio: MotoStock

Sistema de inventario para taller de motos. **23 endpoints** en **68 líneas**.

```
    AURA vs Python/Flask
    ─────────────────────────────────
    Líneas de código    68  vs  450
    Archivos             1  vs   10
    Tokens consumidos   3K  vs  15K
    Tiempo             35m  vs   4h
```

![Dashboard](projects/motostock/screenshots/dashboard.png)

**[→ Ver proyecto completo](projects/motostock/)**

---

## Documentación

| Documento | Contenido |
|-----------|-----------|
| **[SELF-HEALING.md](SELF-HEALING.md)** | Cómo funciona, flujo de trabajo, ejemplos |
| **[SYNTAX.md](SYNTAX.md)** | EBNF formal, quick reference, operadores |
| **[examples/](examples/)** | Ejemplos funcionando |

---

## Comandos

```bash
aura run file.aura       # Ejecutar
aura heal file.aura      # Self-healing
aura serve file.aura     # Servidor HTTP
aura repl                # REPL interactivo
aura check file.aura     # Verificar sintaxis
```

---

## Estado

```
✅ Intérprete completo        ✅ Goals (intención)
✅ REPL interactivo           ✅ Variables de entorno (+env)
✅ Servidor HTTP nativo       ✅ Modularización (+archivo)
✅ Self-healing               ✅ 193 tests
```

---

## Parte de Algo Más Grande

| Proyecto | Pregunta |
|----------|----------|
| [**Yo enseño {con IA}**](https://github.com/bluegin-ush/yo-ense-o-con-IA-) | ¿Qué debe saber un profesional en la era de la IA? |
| **AURA** | Si la IA escribe código, ¿con qué lenguaje? |
| [**IS-IA**](https://github.com/bluegin-ush/IS-IA) | ¿Cómo se construye software con IA? |

---

MIT License

<p align="center">
<strong>AURA: Porque el futuro del código lo escriben máquinas.</strong>
</p>
