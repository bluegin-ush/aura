# Self-Healing en AURA

## El Problema: El Ciclo Tradicional

Cuando usás un agente IA para programar, el flujo es así:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         FLUJO TRADICIONAL                                   │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│   ┌──────────┐      ┌──────────┐      ┌──────────┐      ┌──────────┐       │
│   │  Agente  │ ──▶  │  Código  │ ──▶  │ Ejecutar │ ──▶  │  Error   │       │
│   │  genera  │      │  .py     │      │ python   │      │          │       │
│   └──────────┘      └──────────┘      └──────────┘      └────┬─────┘       │
│                                                               │             │
│        ▲                                                      │             │
│        │                                                      ▼             │
│        │                                              ┌──────────┐          │
│        │◀─────────────────────────────────────────────│  Copiar  │          │
│        │              (ciclo manual)                  │  error   │          │
│                                                       └──────────┘          │
│                                                                             │
│   Cada error requiere: copiar mensaje → pegar al agente → esperar fix      │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

**El problema:**
- Cada error requiere intervención humana
- Copiar/pegar mensajes de error
- Esperar que el agente entienda el contexto
- Volver a ejecutar
- Repetir...

---

## La Solución: Self-Healing

AURA elimina el ciclo manual:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           FLUJO AURA                                        │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│   ┌──────────┐      ┌──────────┐      ┌──────────────────────────────┐     │
│   │  Agente  │ ──▶  │  Código  │ ──▶  │       aura heal              │     │
│   │  genera  │      │  .aura   │      │                              │     │
│   └──────────┘      └──────────┘      │  ┌────────┐    ┌────────┐   │     │
│                                        │  │Ejecutar│───▶│ Error? │   │     │
│                                        │  └────────┘    └───┬────┘   │     │
│                                        │                    │ sí     │     │
│                                        │                    ▼        │     │
│                                        │              ┌──────────┐   │     │
│                                        │              │  Agente  │   │     │
│                                        │              │  repara  │   │     │
│                                        │              └────┬─────┘   │     │
│                                        │                   │         │     │
│                                        │                   ▼         │     │
│                                        │              ┌──────────┐   │     │
│                                        │              │Re-ejecuta│   │     │
│                                        │              └────┬─────┘   │     │
│                                        │                   │         │     │
│                                        │                   ▼         │     │
│                                        │              ┌──────────┐   │     │
│                                        │              │    OK    │   │     │
│                                        │              └──────────┘   │     │
│                                        └──────────────────────────────┘     │
│                                                                             │
│   Todo automático. Sin intervención humana.                                 │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Demo Práctica

### Código con error

```ruby
# broken.aura
goal "calcular el doble de un número"

double(n) = n * 2
main = double(x)   # ← Error: 'x' no está definida
```

### Ejecutar self-healing

```bash
$ aura heal broken.aura

═══════════════════════════════════════════════════════════════
   AURA Self-Healing
═══════════════════════════════════════════════════════════════

📄 File: broken.aura
🎯 Goal: "calcular el doble de un número"

1️⃣  Ejecutando...
❌  Error: Variable no definida: x

2️⃣  Analizando...
🔍  El goal indica que se quiere calcular un doble.
    La variable 'x' no está definida.
    Solución: definir x con un valor numérico.

3️⃣  Fix propuesto:

    --- Original
    +++ Fixed

      goal "calcular el doble de un número"
      double(n) = n * 2
    + x = 21
      main = double(x)

4️⃣  Aplicando fix...
5️⃣  Re-ejecutando...

✅  Resultado: 42

═══════════════════════════════════════════════════════════════
```

### Resultado

El código fue reparado automáticamente. Sin copiar errores. Sin intervención humana.

---

## El Rol del `goal`

El `goal` no es solo un comentario. Es **metadata que el agente usa para razonar**:

### Sin goal
```
Error: Variable no definida: x
→ Agente: "Voy a definir x = 0" (genérico)
```

### Con goal
```
Error: Variable no definida: x
Goal: "calcular el doble de un número"
→ Agente: "El usuario quiere calcular un doble.
           x debe ser un número.
           Voy a definir x = 21 para que el resultado sea 42."
```

El `goal` le da **contexto de intención** al agente, no solo contexto de código.

---

## Comandos

```bash
# Demo de self-healing (no modifica el archivo)
aura heal file.aura

# Aplicar el fix automáticamente
aura heal file.aura --apply

# Usar un proveedor específico
aura heal file.aura --provider claude    # Anthropic API
aura heal file.aura --provider openai    # OpenAI API
aura heal file.aura --provider ollama    # Local (Ollama)

# Ver historial de fixes
aura undo --list

# Revertir último fix
aura undo
```

---

## Arquitectura

```
┌─────────────────┐     Error + Goal      ┌─────────────────┐
│     Runtime     │ ────────────────────▶ │     Agente      │
│      AURA       │                       │  (Claude/GPT)   │
│                 │ ◀──────────────────── │                 │
└────────┬────────┘        Patch          └─────────────────┘
         │
         ▼
┌─────────────────┐
│    Snapshot     │  ← Backup antes de aplicar
│    (backup)     │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  Aplicar Patch  │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│   Verificar     │ ──▶ Si falla, revierte al snapshot
│   ejecución     │
└─────────────────┘
```

**Seguridad:**
- Siempre se crea un snapshot antes de modificar
- Si el fix no funciona, se revierte automáticamente
- Historial de cambios para `aura undo`

---

## Proveedores Soportados

| Proveedor | Comando | Requisito |
|-----------|---------|-----------|
| Mock | `--provider mock` | Ninguno (demo) |
| Claude | `--provider claude` | `ANTHROPIC_API_KEY` |
| OpenAI | `--provider openai` | `OPENAI_API_KEY` |
| Ollama | `--provider ollama` | Ollama corriendo local |

```bash
# Sin API key (usa mock para demo)
aura heal broken.aura

# Con Claude
ANTHROPIC_API_KEY=sk-xxx aura heal broken.aura --provider claude

# Con Ollama local (gratis)
aura heal broken.aura --provider ollama
```

---

## Por Qué Esto Importa

### Para el programador
- No más copiar/pegar errores
- Ciclo de desarrollo más rápido
- El agente tiene contexto completo (código + goal + error)

### Para el agente
- Acceso directo al error real
- Conoce la intención (`goal`)
- Puede verificar si su fix funciona
- Loop cerrado de feedback

### Para el costo
- Menos tokens desperdiciados en ida y vuelta
- Fixes más precisos = menos iteraciones
- Automatización reduce tiempo humano

---

## Ejemplo Avanzado: API con múltiples errores

```ruby
# api.aura
+http +json

goal "API que obtiene usuarios y los formatea"

get_user(id) = : r = http.get(base_url ++ "/users/" ++ id); json.parse(r.body)
format(user) = "Name: {user.name}, Email: {user.emal}"   # typo: emal
main = : users = get_user(1); format(users)              # users vs user
```

```bash
$ aura heal api.aura --apply

Error 1: Variable no definida: base_url
Fix: base_url = "https://jsonplaceholder.typicode.com"

Error 2: Campo no existe: user.emal
Fix: Corregido a user.email

Error 3: format espera user, recibe users
Fix: Renombrado users → user

✅ Resultado: "Name: Leanne Graham, Email: Sincere@april.biz"
```

El agente corrigió **3 errores en cadena**, automáticamente.

---

## Resumen

| Tradicional | AURA Self-Healing |
|-------------|-------------------|
| Error → Copiar → Pegar → Esperar → Repetir | Error → Fix automático |
| Contexto fragmentado | Código + Goal + Error juntos |
| Humano en el loop | Loop cerrado automático |
| Múltiples iteraciones | Una ejecución |

**El futuro del desarrollo es que las máquinas corrijan su propio código.**
