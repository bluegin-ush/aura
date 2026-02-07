# MotoStock - Especificación Técnica

## Visión General

Sistema de gestión de inventario para taller de motos, implementado como caso de estudio de AURA.

## Arquitectura

```
┌─────────────────────────────────────────────────────────────┐
│                      Frontend (htmx)                        │
│                    HTML + CSS + htmx                        │
└─────────────────────────┬───────────────────────────────────┘
                          │ HTTP/JSON
┌─────────────────────────▼───────────────────────────────────┐
│                    Backend (AURA)                           │
│              motostock.aura (68 líneas)                     │
└─────────────────────────┬───────────────────────────────────┘
                          │ SQL
┌─────────────────────────▼───────────────────────────────────┐
│                    SQLite Database                          │
│         parts, motos, orders, order_items                   │
└─────────────────────────────────────────────────────────────┘
```

## Modelo de Datos

### Entidad-Relación

```
┌──────────────┐       ┌──────────────┐
│    Parts     │       │    Motos     │
├──────────────┤       ├──────────────┤
│ id           │       │ id           │
│ code         │       │ plate        │
│ name         │       │ brand        │
│ brand        │       │ model        │
│ price        │       │ year         │
│ stock        │       │ owner_name   │
│ min_stock    │       │ owner_phone  │
└──────┬───────┘       └──────┬───────┘
       │                      │
       │ part_id              │ moto_id
       │                      │
       ▼                      ▼
┌──────────────┐       ┌──────────────┐
│ Order_Items  │◄──────│   Orders     │
├──────────────┤       ├──────────────┤
│ id           │       │ id           │
│ order_id     │       │ moto_id      │
│ part_id      │       │ description  │
│ quantity     │       │ status       │
│ unit_price   │       │ created_at   │
└──────────────┘       └──────────────┘
```

## API Endpoints (23 total)

### Parts (7)
- `GET /parts` - Listar
- `GET /part/:id` - Obtener
- `POST /part/:code/:name/:brand/:price/:stock/:min_stock` - Crear
- `PUT /part/:id/:price` - Actualizar
- `DELETE /part/:id` - Eliminar
- `GET /partsLowStock` - Stock bajo
- `GET /partsSearch?q=` - Buscar

### Motos (6)
- `GET /motos` - Listar
- `GET /moto/:id` - Obtener
- `POST /moto/:plate/:brand/:model/:year/:owner_name/:owner_phone` - Crear
- `PUT /moto/:id/:owner_name/:owner_phone` - Actualizar
- `DELETE /moto/:id` - Eliminar
- `GET /motoOrders/:id` - Historial

### Orders (6)
- `GET /orders` - Listar
- `GET /order/:id` - Obtener
- `POST /order/:moto_id/:description` - Crear
- `PUT /orderStatus/:id/:status` - Estado
- `DELETE /order/:id` - Eliminar
- `GET /orderTotal/:id` - Total

### Order Items (3)
- `GET /orderItems/:id` - Listar
- `POST /orderItem/:id/:part_id/:quantity` - Agregar

### Reports (3)
- `GET /reportsInventory` - Inventario
- `GET /reportsLowStock` - Stock bajo
- `GET /reportsMonthly` - Mensual

## Reglas de Negocio

1. **Stock Automático**: Al agregar item → descuenta stock
2. **Precio Histórico**: Se guarda precio al momento de uso
3. **Alertas**: stock < min_stock → aparece en reportes
4. **Estados**: pending → in_progress → completed

## Stack Tecnológico

| Capa | Tecnología | Tamaño |
|------|------------|--------|
| Frontend | htmx + Pico CSS | ~50 KB |
| Backend | AURA | 68 líneas |
| Database | SQLite | ~32 KB |
| **Total** | | **< 100 KB** |

## Métricas vs Python/Flask

| Métrica | Python/Flask | AURA | Reducción |
|---------|--------------|------|-----------|
| Líneas de código | ~450 | 68 | **85%** |
| Archivos | 8-10 | 2 | **80%** |
| Dependencias | 15+ | 0 | **100%** |
| Tiempo desarrollo | ~4h | 35min | **85%** |
| Tokens LLM | ~15K | ~3K | **80%** |
