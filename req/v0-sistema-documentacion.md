# v0 - Sistema de Documentación de Requisitos

## Propósito

Esta carpeta `req/` contiene el análisis incremental y las especificaciones del lenguaje AURA (Agent-Unified Runtime Architecture).

## Convención de versionado

```
v0-*  → Meta-documentación (este archivo)
v1-*  → Visión y principios fundamentales
v2-*  → Especificación de sintaxis
v3-*  → Especificación del runtime
v4-*  → Protocolo agente-runtime
v5+   → Extensiones y refinamientos
```

## Formato de archivos

```
req/
├── v0-sistema-documentacion.md     # Este archivo
├── v1-vision-principios.md         # Por qué AURA existe
├── v2-sintaxis.md                  # Gramática y ejemplos
├── v3-runtime.md                   # Cómo ejecuta el código
├── v4-agente-bridge.md             # Comunicación con agentes IA
└── ...
```

## Contexto del proyecto

**Fecha de inicio:** 2026-02-04

**Objetivo:** Crear un lenguaje de programación ultra-performante para agentes de IA, donde:
- Los humanos rara vez tocarán el código
- Los agentes serán los principales escritores y lectores
- El runtime puede comunicarse con agentes para auto-reparación y expansión

**Lenguaje de implementación:** Rust

## Cómo usar esta documentación

1. Cada versión es incremental y se construye sobre las anteriores
2. Los documentos son la fuente de verdad para decisiones de diseño
3. Antes de implementar algo, debe estar especificado aquí
4. Los agentes pueden usar estos docs como contexto para contribuir
