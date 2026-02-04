# v1 - Visión y Principios Fundamentales

## Qué es AURA

**AURA** (Agent-Unified Runtime Architecture) es un lenguaje de programación diseñado específicamente para ser escrito y leído por agentes de IA, no por humanos.

## El problema

Los lenguajes actuales fueron diseñados para humanos. Cuando un agente de IA los usa, enfrenta:

### 1. Contexto fragmentado
```
Para modificar UNA función necesito:
├── Leer el archivo principal
├── Leer los tipos/interfaces (otro archivo)
├── Leer las dependencias que usa (otros archivos)
├── Leer los tests (otra carpeta)
├── Leer la config (package.json, tsconfig, etc.)
└── Leer ejemplos de uso (docs o tests)

= 6+ archivos para UN cambio
= Miles de tokens de contexto
= Alta probabilidad de error
```

### 2. Boilerplate repetitivo
```python
# Esto se repite en CADA archivo:
import os
import sys
import json
import logging
from typing import Optional, List, Dict
from pathlib import Path

logger = logging.getLogger(__name__)
```

### 3. Errores inútiles
```
TypeError: Cannot read property 'map' of undefined
    at Object.<anonymous> (/app/src/utils/helpers.js:47:23)
    ... 15 líneas más de ruido ...

# ¿Qué fue undefined? ¿Por qué? ¿Cómo lo arreglo?
```

### 4. Múltiples formas de hacer lo mismo
```javascript
array.map(x => x * 2)
array.map(function(x) { return x * 2 })
array.map((x) => { return x * 2 })
_.map(array, x => x * 2)
// Resultado: código inconsistente
```

### 5. Indentación como sintaxis
```python
def foo():
    if condition:
        for item in items:
            if other:
                do_thing()  # ¿4 espacios o 1 tab? Error invisible
```

### 6. Async implícito
```javascript
const data = fetchData()  // Oops, es una Promise
console.log(data.name)    // undefined, sin error
```

### 7. Configuración explosiva
```
proyecto-simple/
├── package.json
├── package-lock.json
├── tsconfig.json
├── .eslintrc.js
├── .prettierrc
├── jest.config.js
├── .env
├── docker-compose.yml
└── ... el código está en algún lugar
```

### 8. Null/undefined everywhere
```python
if x is not None:
    if x.data is not None:
        if x.data.items is not None:
            # finalmente puedo trabajar
```

## La solución: Diseñar para agentes

### Principios de diseño

| # | Principio | Razón |
|---|-----------|-------|
| 1 | **Mínimos tokens** | Cada token cuesta $ y latencia |
| 2 | **Cero ambigüedad** | Una sola forma de escribir cada cosa |
| 3 | **Autocontenido** | No saltar entre archivos |
| 4 | **Errores parseables** | JSON estructurado, no texto |
| 5 | **Parseo incremental** | Validar mientras se genera |
| 6 | **Sin decisiones de estilo** | No hay "formato", solo sintaxis |

### Fortalezas resultantes

| Debilidad actual | Fortaleza AURA |
|------------------|----------------|
| Contexto fragmentado | Todo en un lugar: tipos, impl, tests, docs juntos |
| Imports/boilerplate | Capacidades implícitas: `+http` y listo |
| Errores crípticos | Errores accionables en JSON |
| Múltiples formas | Una forma canónica única |
| Indentación frágil | Delimitadores explícitos pero mínimos |
| Async implícito | Efectos explícitos con `!` |
| Config explosiva | Convención > configuración |
| Null hell | Nullabilidad en el tipo: `s` vs `s?` |

## La innovación: Runtime con Agente en el Loop

AURA no es solo un lenguaje, es un **sistema de programación vivo**.

```
┌─────────────────────────────────────────────────────────────┐
│                      AURA RUNTIME                           │
│                                                             │
│  ┌─────────┐    ┌─────────┐    ┌─────────┐                │
│  │  Code   │───▶│ Execute │───▶│ Result  │                │
│  └─────────┘    └────┬────┘    └─────────┘                │
│                      │                                      │
│                      ▼ (error/expansión/duda)              │
│                 ┌─────────┐                                 │
│                 │  Agent  │◀──── API (Claude, OpenAI, etc) │
│                 │  Bridge │                                 │
│                 └────┬────┘                                 │
│                      │                                      │
│                      ▼ (fix/código nuevo/decisión)         │
│                 ┌─────────┐                                 │
│                 │  Hot    │                                 │
│                 │ Reload  │                                 │
│                 └─────────┘                                 │
└─────────────────────────────────────────────────────────────┘
```

### Capacidades del runtime inteligente

1. **Self-healing**: Detecta errores, llama al agente, aplica fix en caliente
2. **Expansión bajo demanda**: Función no existe → agente la genera → continúa
3. **Validación inteligente**: El agente decide qué hacer con inputs inválidos
4. **Debugging asistido**: El agente puede inspeccionar y modificar estado
5. **Evolución continua**: El programa mejora basado en patrones de uso

## Dominio inicial: Aplicaciones Web

El 90% de lo que los agentes escriben es:
```
CRUD + Auth + Validación + APIs + DB + Realtime
```

AURA optimiza esto dramáticamente:

**Express + Prisma + JWT + Zod típico: ~400 líneas, 8 archivos, ~2000 tokens**

**AURA equivalente: ~4 líneas, 1 archivo, ~50 tokens**

**Reducción: 98% menos tokens**

## Métricas de éxito

1. **Tokens por funcionalidad**: 10x menos que equivalente en Python/JS/TS
2. **Archivos por proyecto**: Idealmente 1 (monolito legible)
3. **Tiempo de corrección de errores**: El agente puede auto-corregir en 1 ciclo
4. **Curva de aprendizaje para agentes**: Inmediata (sintaxis mínima)

## Implementación

- **Lenguaje:** Rust
- **Parser:** logos + chumsky
- **Target inicial:** Bytecode interpretado, luego LLVM/WASM
- **Capacidades iniciales:** +http +json +db +auth

## Referencias

Este documento captura la discusión fundacional del proyecto AURA.
Próximo: v2-sintaxis.md con la especificación completa de la gramática.
