# MotoStock - Sistema de Gestión de Stock para Taller de Motos

## Visión

API REST simple para gestionar el inventario de repuestos y servicios de un taller de motos, implementada 100% en AURA.

---

## Entidades

### Repuesto (Part)
| Campo | Tipo | Descripción |
|-------|------|-------------|
| id | int | Identificador único |
| code | string | Código interno (ej: "ACE-001") |
| name | string | Nombre del repuesto |
| brand | string | Marca (Honda, Yamaha, etc.) |
| price | float | Precio unitario |
| stock | int | Cantidad en inventario |
| min_stock | int | Stock mínimo para alerta |

### Moto (Motorcycle)
| Campo | Tipo | Descripción |
|-------|------|-------------|
| id | int | Identificador único |
| plate | string | Patente |
| brand | string | Marca |
| model | string | Modelo |
| year | int | Año |
| owner_name | string | Nombre del dueño |
| owner_phone | string | Teléfono del dueño |

### Orden de Trabajo (WorkOrder)
| Campo | Tipo | Descripción |
|-------|------|-------------|
| id | int | Identificador único |
| moto_id | int | ID de la moto |
| description | string | Descripción del trabajo |
| status | string | pending/in_progress/completed |
| created_at | string | Fecha de creación |

### Item de Orden (OrderItem)
| Campo | Tipo | Descripción |
|-------|------|-------------|
| id | int | Identificador único |
| order_id | int | ID de la orden |
| part_id | int | ID del repuesto |
| quantity | int | Cantidad usada |
| unit_price | float | Precio al momento |

---

## Endpoints API

### Repuestos
```
GET    /parts              # Listar todos
GET    /parts/:id          # Obtener uno
POST   /parts              # Crear
PUT    /parts/:id          # Actualizar
DELETE /parts/:id          # Eliminar
GET    /parts/low-stock    # Repuestos bajo stock mínimo
GET    /parts/search?q=    # Buscar por nombre/código
```

### Motos
```
GET    /motos              # Listar todas
GET    /motos/:id          # Obtener una
POST   /motos              # Crear
PUT    /motos/:id          # Actualizar
GET    /motos/:id/orders   # Historial de órdenes
```

### Órdenes de Trabajo
```
GET    /orders             # Listar todas
GET    /orders/:id         # Obtener una con items
POST   /orders             # Crear nueva orden
PUT    /orders/:id/status  # Cambiar estado
POST   /orders/:id/items   # Agregar item (descuenta stock)
GET    /orders/:id/total   # Calcular total
```

### Reportes
```
GET    /reports/inventory  # Valor total del inventario
GET    /reports/low-stock  # Lista de repuestos a reponer
GET    /reports/monthly    # Resumen del mes
```

---

## Reglas de Negocio

1. **Stock automático**: Al agregar item a orden, se descuenta del stock
2. **Alerta stock bajo**: Cuando stock < min_stock, aparece en reporte
3. **Historial de precios**: Se guarda el precio al momento de usar el repuesto
4. **Estados de orden**: pending → in_progress → completed

---

## Stack Técnico

- **Lenguaje**: AURA
- **Base de datos**: SQLite (archivo local)
- **Servidor HTTP**: Por implementar en AURA
- **Formato**: JSON para request/response

---

## Estructura de Archivos

```
projects/motostock/
├── REQUIREMENTS.md      # Este archivo
├── motostock.aura       # Código principal
├── schema.sql           # Schema de base de datos
├── test_api.sh          # Tests de la API
└── README.md            # Documentación de uso
```

---

## Fases de Implementación

### Fase 1: Core (MVP)
- [ ] Schema de base de datos
- [ ] CRUD de repuestos
- [ ] CRUD de motos
- [ ] Búsqueda básica

### Fase 2: Órdenes
- [ ] Crear órdenes de trabajo
- [ ] Agregar items a órdenes
- [ ] Descuento automático de stock
- [ ] Cambio de estado

### Fase 3: Reportes
- [ ] Inventario total
- [ ] Stock bajo
- [ ] Resumen mensual

---

## Ejemplo de Uso

```bash
# Crear repuesto
curl -X POST http://localhost:8080/parts \
  -d '{"code":"ACE-001","name":"Aceite 10W40","brand":"Motul","price":25.50,"stock":20,"min_stock":5}'

# Buscar repuestos
curl http://localhost:8080/parts/search?q=aceite

# Crear orden de trabajo
curl -X POST http://localhost:8080/orders \
  -d '{"moto_id":1,"description":"Service 10000km"}'

# Agregar item a orden
curl -X POST http://localhost:8080/orders/1/items \
  -d '{"part_id":1,"quantity":2}'

# Ver total de orden
curl http://localhost:8080/orders/1/total
```

---

## Nota

Este proyecto sirve como demostración de que AURA puede manejar aplicaciones reales de negocio con una fracción del código que requeriría Python/Node.js.
