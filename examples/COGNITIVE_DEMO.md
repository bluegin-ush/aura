# Demo Cognitivo: Monitor de Sensores IoT

Este ejemplo demuestra el **runtime cognitivo** de AURA v2.0 — la capacidad del intérprete de observar, razonar y **auto-reparar** código en tiempo de ejecución.

---

## El escenario

Un sistema IoT que lee sensores de temperatura, detecta anomalías térmicas y genera alertas. El código tiene un **bug intencional**: la variable `umbral_temp` nunca se define.

```
es_anomalia(temp) = temp > umbral_temp   # <-- umbral_temp no existe
```

Sin el runtime cognitivo, el programa crashea. Con él, el error se detecta, se delibera una solución, se aplica el fix, y la ejecución se reintenta automáticamente.

---

## Ejecutar

### Sin cognitive (crash esperado)

```bash
aura run examples/cognitive_demo.aura
```

```
Sensor TH-01: 22.5C
Sensor TH-02: 38.7C
Runtime error: Variable no definida: umbral_temp
```

El programa imprime los dos primeros sensores, luego llama a `es_anomalia(38.7)` que intenta evaluar `temp > umbral_temp`. Como `umbral_temp` no está definido, crashea.

### Con cognitive (auto-reparación)

```bash
aura run --cognitive --provider mock examples/cognitive_demo.aura
```

```
Cognitive mode: provider=mock
Sensor TH-01: 22.5C
Sensor TH-02: 38.7C
Sensor TH-01: 22.5C
Sensor TH-02: 38.7C
ALERTA: sensor TH-02 reporta 38.7C
Analisis completo
38.7
  [1 fix(es) applied, 1 retries]
```

El runtime cognitivo intercepta el error, delibera un fix (reemplazar `umbral_temp` con `35.0`), lo aplica al código fuente, y reintenta. En la segunda ejecución todo funciona.

---

## Las 6 primitivas cognitivas en acción

### 1. `goal` — Declarar intenciones

```
goal "monitorear todos los sensores"
goal "detectar anomalias termicas" check lecturas != nil
```

Los goals declaran qué quiere lograr el programa. El segundo tiene un **check activo**: la expresión `lecturas != nil` se evalúa periódicamente durante la ejecución. Si falla, el runtime cognitivo delibera sobre la desalineación.

### 2. `invariant` — Restricciones de seguridad

```
invariant len(lecturas) >= 0
```

Las invariantes definen condiciones que **nunca** deben violarse. A diferencia de los goals (aspiracionales), las invariantes son restricciones duras. Si un fix propuesto viola una invariante, se rechaza.

### 3. `observe` — Monitorear variables

```
observe lecturas
```

Registra la variable `lecturas` para monitoreo. Cada vez que cambia de valor, el runtime cognitivo recibe una notificación `ValueChanged`. Esto alimenta el buffer de observaciones que se incluye en cada deliberación.

### 4. `expect` — Verificar condiciones

```
expect len(lecturas) > 0 "sin datos de sensores"
expect a2 "se esperaba anomalia en {s2.sensor}"
```

Similar a un assert, pero **no detiene la ejecución**. Si la condición falla, el runtime cognitivo delibera sobre qué hacer: continuar, inyectar un valor, o generar un fix. El mensaje es contexto para la deliberación.

### 5. `reason` — Deliberación explícita

```
accion = reason "sensor {s2.sensor} en {s2.temp}C, que accion tomar?"
```

Invoca al agente cognitivo con una pregunta abierta. El agente considera las observaciones acumuladas, los goals activos y las invariantes, y retorna una decisión. Es el mecanismo para que el programa "piense en voz alta".

### 6. Auto-reparación (implícita)

No es una keyword — es el comportamiento emergente del sistema. Cuando `eval()` produce un `RuntimeError`:

1. El VM detecta que el cognitive runtime está activo
2. Llama a `deliberate(TechnicalError { error })` con el error y el código fuente
3. El agente analiza el error y propone un fix: nuevo código fuente corregido
4. El fix pasa por validación de seguridad (no modifica goals, no excede tamaño máximo, parsea correctamente)
5. El runner aplica el fix y reintenta la ejecución

---

## Flujo de ejecución detallado

```
                    ┌──────────────┐
                    │  Código .aura│
                    └──────┬───────┘
                           │
                    ┌──────▼───────┐
                    │   Tokenizar  │
                    │   + Parsear  │
                    └──────┬───────┘
                           │
                    ┌──────▼───────┐
              ┌────►│  VM con      │
              │     │  Cognitive   │
              │     │  Runtime     │
              │     └──────┬───────┘
              │            │
              │     ┌──────▼───────┐
              │     │  eval(main)  │
              │     └──────┬───────┘
              │            │
              │      Error: umbral_temp
              │      no definida
              │            │
              │     ┌──────▼───────┐
              │     │  deliberate  │
              │     │  (Technical  │
              │     │   Error)     │
              │     └──────┬───────┘
              │            │
              │     ┌──────▼───────┐
              │     │ MockProvider  │
              │     │ genera fix:  │
              │     │ umbral_temp  │
              │     │   → 35.0     │
              │     └──────┬───────┘
              │            │
              │     ┌──────▼───────┐
              │     │  Validar fix │
              │     │ (goals, size,│
              │     │  parseable)  │
              │     └──────┬───────┘
              │            │ OK
              │     ┌──────▼───────┐
              │     │ Aplicar fix  │
              │     │ al source    │
              │     └──────┬───────┘
              │            │
              └────────────┘
                   Reintentar
                           │
                    ┌──────▼───────┐
                    │  Ejecución   │
                    │  exitosa     │
                    │  → 38.7      │
                    └──────────────┘
```

---

## Anatomía del código

```
+http +json                              # Capacidades requeridas
```

Las capacidades se declaran al inicio. Habilitan módulos (`http.get`, `json.parse`, etc.).

```
goal "monitorear todos los sensores"
goal "detectar anomalias termicas" check lecturas != nil
```

Goals: el "para qué" del programa. Con `check`, se evalúan activamente.

```
invariant len(lecturas) >= 0
```

Invariante: restricción de seguridad que nunca debe violarse.

```
obtener_lecturas() = [{sensor: "TH-01", temp: 22.5, humedad: 45}, ...]
```

Función pura que retorna datos. En AURA, `f() = expr` define una función. Los records son `{clave: valor}`.

```
es_anomalia(temp) = temp > umbral_temp
```

El bug: `umbral_temp` no existe en ningún scope. Sin cognitive, esto crashea. Con cognitive, el runtime reemplaza `umbral_temp` con `35.0`.

```
formatear_alerta(s) = "ALERTA: sensor {s.sensor} reporta {s.temp}C"
```

String interpolation con acceso a campos de records: `{s.campo}`.

```
main = : lecturas = obtener_lecturas();
         observe lecturas;
         expect len(lecturas) > 0 "sin datos de sensores";
         s1 = first(lecturas);
         s2 = first(tail(lecturas));
         print("Sensor {s1.sensor}: {s1.temp}C");
         print("Sensor {s2.sensor}: {s2.temp}C");
         a2 = es_anomalia(s2.temp);
         expect a2 "se esperaba anomalia en {s2.sensor}";
         alerta = formatear_alerta(s2);
         print(alerta);
         accion = reason "sensor {s2.sensor} en {s2.temp}C, que accion tomar?";
         print("Analisis completo");
         s2.temp
```

El bloque principal (`: expr; expr; ...` es un bloque secuencial). Usa `first()`, `tail()`, `len()` y `print()` — funciones built-in del lenguaje.

---

## Providers

El flag `--provider` selecciona quién delibera:

| Provider | Descripción | Uso |
|----------|------------|-----|
| `mock` | Respuestas deterministas sin red | Desarrollo, testing, demos |
| `claude` | Anthropic API (requiere `ANTHROPIC_API_KEY`) | Producción |
| `ollama` | Modelo local via Ollama | Offline, privacidad |

El `mock` genera fixes inteligentes basados en patrones del error: reconoce variables con nombres como `umbral_*`, `threshold_*`, `max_*`, `timeout_*`, etc., y les asigna valores por defecto sensibles.

---

## Por qué importa

En un lenguaje tradicional, un bug es un crash. En AURA con runtime cognitivo, un bug es una **oportunidad de deliberación**:

1. **El programa declara intenciones** (goals) y restricciones (invariants)
2. **El runtime observa** la ejecución (observe, expect)
3. **Cuando algo falla**, delibera con un agente IA en lugar de crashear
4. **El agente propone una solución** dentro de las restricciones de seguridad
5. **El runtime valida y aplica** el fix, reintentando la ejecución

Esto no es "try/catch con IA". Es un bucle cognitivo donde el programa y el agente colaboran en tiempo de ejecución, con barandas de seguridad (goals inmutables, invariantes, límite de reintentos, validación de fixes).
