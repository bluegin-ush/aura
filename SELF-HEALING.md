# Self-healing en AURA

## El problema: el ciclo tradicional

Cuando usás un agente para programar, el flujo es así:

```
    Flujo tradicional
    ─────────────────────────────────────────────────────────────

    Agente ──▶ Código ──▶ Ejecutar ──▶ Error
      ▲                                  │
      │                                  ▼
      └──────── Copiar error ◀───── Humano

    Cada error requiere: copiar mensaje, pegar al agente, esperar, repetir.
```

El problema:
- Cada error requiere intervención humana
- Copiar/pegar mensajes de error
- El agente pierde contexto entre iteraciones
- Múltiples ciclos hasta que funciona

---

## La solución: self-healing

AURA elimina el ciclo manual:

```
    Flujo AURA
    ─────────────────────────────────────────────────────────────

    Agente ──▶ Código ──▶ aura heal ──┐
                              │       │
                         Ejecutar     │
                              │       │
                         ¿Error? ─────┤
                              │       │
                         Reparar ◀────┘
                              │
                         Re-ejecutar
                              │
                             OK

    Todo automático. Sin intervención humana.
```

---

## Ejemplo práctico: variable de configuración faltante

Este es uno de los errores más comunes cuando un agente genera código. El agente asume que ciertas variables ya existen.

### Código generado por el agente

```ruby
# api.aura
+http +json

goal "consultar usuarios de la API"

get_users = : r = http.get(api_url ++ "/users"); json.parse(r.body)
main = get_users()
```

El agente generó código que usa `api_url`, pero nunca la definió.

### Ejecutar con self-healing

```
$ aura heal api.aura

Archivo: api.aura
Goal: "consultar usuarios de la API"

Ejecutando...
Error: variable no definida: api_url

Analizando contexto...
- Se intenta concatenar api_url con "/users"
- El goal menciona "API"
- Se necesita una URL base

Fix propuesto:

    +http +json
    goal "consultar usuarios de la API"
  + api_url = "https://api.example.com"
    get_users = : r = http.get(api_url ++ "/users"); json.parse(r.body)
    main = get_users()

Aplicando...
Re-ejecutando...

Resultado: [{id: 1, name: "Alice"}, {id: 2, name: "Bob"}]
```

El agente reparó el código agregando la definición faltante.

---

## El rol del goal

El `goal` no es un comentario. Es metadata que el agente usa para razonar sobre la intención.

### Sin goal
```
Error: variable no definida: api_url
Agente: "Voy a definir api_url = nil" (genérico, no útil)
```

### Con goal
```
Error: variable no definida: api_url
Goal: "consultar usuarios de la API"
Agente: "El usuario quiere consultar una API.
         api_url debe ser una URL.
         Voy a definir api_url con un valor apropiado."
```

El goal le da contexto de intención, no solo contexto de código.

---

## Otros errores comunes que se reparan

### Typo en campo de respuesta

```ruby
goal "mostrar nombre del usuario"
main = : user = get_user(1); user.username  # API devuelve 'name', no 'username'
```

Fix: cambia `user.username` a `user.name`

### Capacidad no declarada

```ruby
# Falta +http
get_data = http.get("https://api.com/data")
```

Fix: agrega `+http` al inicio

### División por cero potencial

```ruby
promedio(lista) = sum(lista) / len(lista)  # Si lista está vacía, divide por 0
```

Fix: agrega validación o valor por defecto

---

## Arquitectura

```
    Runtime          Error + Goal          Agente
       │ ─────────────────────────────────▶ │
       │                                    │
       │ ◀───────────── Patch ───────────── │
       │
       ▼
    Snapshot (backup)
       │
       ▼
    Aplicar patch
       │
       ▼
    Verificar ──────▶ Si falla, revierte al snapshot
```

Seguridad:
- Siempre se crea backup antes de modificar
- Si el fix no funciona, se revierte
- Historial disponible con `aura undo`

---

## Comandos

```bash
# Ver el fix propuesto (no modifica)
aura heal archivo.aura

# Aplicar el fix
aura heal archivo.aura --apply

# Usar proveedor específico
aura heal archivo.aura --provider claude
aura heal archivo.aura --provider openai
aura heal archivo.aura --provider ollama

# Historial de cambios
aura undo --list

# Revertir último fix
aura undo
```

---

## Proveedores

| Proveedor | Variable de entorno | Notas |
|-----------|---------------------|-------|
| mock | ninguna | para demos |
| claude | `ANTHROPIC_API_KEY` | Anthropic API |
| openai | `OPENAI_API_KEY` | OpenAI API |
| ollama | ninguna | requiere Ollama local |

---

## Features Agent-Native

### expect: verificación de intención

El `expect` verifica condiciones sin crashear. Si falla, registra la "desviación de intención":

```ruby
goal "todos los usuarios activos"

main = : users = get_users(); expect users.all(u -> u.active); users
```

Si el expect falla, el agente sabe que el resultado no cumple la intención.

### invariant: constraints para healing

Los invariants son reglas que el healing NO puede violar:

```ruby
invariant api_url != "https://production.com"
invariant !contains(code, "mock_data")

goal "consultar API"
main = http.get(api_url ++ "/users")
```

Si el agente propone un fix que viola un invariant, se rechaza.

### @self_heal: healing automático

Marca funciones para que se reparen automáticamente en runtime:

```ruby
@self_heal
main = get_users() |> filter(active) |> count

@self_heal(max_attempts: 5, mode: "technical")
risky_operation() = http.get(unstable_url)
```

Modos:
- `technical`: corrige errores de sintaxis y tipos
- `semantic`: corrige basándose en goals
- `auto`: elige el mejor modo

### memoria de healing

El sistema recuerda patrones de errores y fixes:

```bash
# Ver patrones aprendidos
aura memory list

# Gestionar defaults del proyecto
aura memory defaults --set api_url=https://api.example.com
```

Archivo `.aura-memory.json` (local, no se sube a git).

---

## Resumen

| Tradicional | AURA |
|-------------|------|
| Error, copiar, pegar, esperar, repetir | Error, fix automático |
| Contexto fragmentado | Código + goal + error juntos |
| Humano en cada iteración | Loop cerrado |
| Múltiples ciclos | Una ejecución |
| Sin memoria | Aprende patrones |
| Sin constraints | Invariants respetados |

El futuro del desarrollo es que las máquinas corrijan su propio código.
