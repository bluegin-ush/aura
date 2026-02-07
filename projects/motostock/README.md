# MotoStock - Caso de Estudio AURA

Sistema completo de gestión de inventario para taller de motos.

**Desarrollado en 35 minutos** por un agente IA usando AURA.

## Estructura del Proyecto

```
motostock/
├── spec/           # Especificación técnica
│   ├── SPEC.md
│   └── REQUIREMENTS.md
├── back/           # Backend AURA
│   ├── motostock.aura   # API (68 líneas)
│   ├── init.aura        # Inicialización DB
│   ├── schema.sql       # Schema referencia
│   └── test_api.sh      # Tests (26 tests)
├── front/          # Frontend htmx
│   ├── index.html
│   ├── style.css
│   └── app.js
├── screenshots/    # Capturas de la app
└── README.md
```

## Métricas vs Python/Flask

```
┌────────────────────┬─────────────┬────────┬───────────┐
│      Métrica       │ Python/Flask│  AURA  │ Reducción │
├────────────────────┼─────────────┼────────┼───────────┤
│ Líneas de código   │    ~450     │   68   │    85%    │
│ Archivos backend   │    8-10     │    2   │    80%    │
│ Dependencias       │    15+      │    0   │   100%    │
│ Tiempo desarrollo  │    ~4h      │  35min │    85%    │
│ Tokens LLM         │   ~15K      │   ~3K  │    80%    │
└────────────────────┴─────────────┴────────┴───────────┘
```

## Inicio Rápido

```bash
cd projects/motostock/back

# Inicializar base de datos
aura run init.aura

# Iniciar servidor API (puerto 8081)
aura serve motostock.aura --port 8081

# Ejecutar tests
./test_api.sh
```

Para el frontend, abrir `front/index.html` en un navegador.

## API Endpoints (23 total)

### Repuestos (7)
| Método | Endpoint | Descripción |
|--------|----------|-------------|
| GET | `/parts` | Listar |
| GET | `/part/:id` | Obtener |
| POST | `/part/:code/:name/:brand/:price/:stock/:min_stock` | Crear |
| PUT | `/part/:id/:price` | Actualizar |
| DELETE | `/part/:id` | Eliminar |
| GET | `/partsLowStock` | Stock bajo |
| GET | `/partsSearch?q=` | Buscar |

### Motos (6)
| Método | Endpoint | Descripción |
|--------|----------|-------------|
| GET | `/motos` | Listar |
| GET | `/moto/:id` | Obtener |
| POST | `/moto/:plate/:brand/:model/:year/:owner_name/:owner_phone` | Crear |
| PUT | `/moto/:id/:owner_name/:owner_phone` | Actualizar |
| DELETE | `/moto/:id` | Eliminar |
| GET | `/motoOrders/:id` | Historial |

### Órdenes (7)
| Método | Endpoint | Descripción |
|--------|----------|-------------|
| GET | `/orders` | Listar |
| GET | `/order/:id` | Obtener |
| POST | `/order/:moto_id/:description` | Crear |
| PUT | `/orderStatus/:id/:status` | Estado |
| DELETE | `/order/:id` | Eliminar |
| GET | `/orderItems/:id` | Ver items |
| POST | `/orderItem/:id/:part_id/:quantity` | Agregar item |
| GET | `/orderTotal/:id` | Total |

### Reportes (3)
| Método | Endpoint | Descripción |
|--------|----------|-------------|
| GET | `/reportsInventory` | Valor inventario |
| GET | `/reportsLowStock` | Alertas stock |
| GET | `/reportsMonthly` | Resumen mes |

## Reglas de Negocio

1. **Stock Automático**: Al agregar item a orden → descuenta stock
2. **Precio Histórico**: Se guarda precio al momento de uso
3. **Alertas**: stock < min_stock → aparece en reportes
4. **Estados**: pending → in_progress → completed

## Stack Tecnológico

| Capa | Tecnología | Tamaño |
|------|------------|--------|
| Frontend | htmx + Pico CSS | ~5 KB |
| Backend | AURA | 68 líneas |
| Database | SQLite | ~32 KB |

## Screenshots

Ver carpeta `screenshots/` para capturas de la interfaz.

## Por Qué AURA

Este proyecto demuestra las ventajas de AURA para desarrollo con agentes IA:

1. **Menos código** = Menos tokens = Menor costo
2. **Sin dependencias** = Sin errores de instalación
3. **Sintaxis mínima** = Menor tasa de errores
4. **Self-contained** = 1-2 archivos vs 10+

Un agente IA puede generar, entender y modificar este proyecto completo en una fracción del tiempo y costo que requeriría con Python o Node.js.
