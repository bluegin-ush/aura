# MotoStock - API de Gestión de Stock

API REST para gestión de inventario de taller de motos, construida con AURA.

## Requisitos

- AURA compilado con soporte para `aura serve`
- curl (para tests)

## Inicio Rápido

```bash
cd projects/motostock
aura run init.aura              # Inicializar base de datos
aura serve motostock.aura --port 8081  # Iniciar servidor
./test_api.sh                   # Ejecutar tests (26 tests)
```

## Endpoints

### Repuestos (Parts)
| Método | Endpoint | Descripción |
|--------|----------|-------------|
| GET | `/parts` | Listar todos |
| GET | `/part/:id` | Obtener uno |
| POST | `/part/:code/:name/:brand/:price/:stock/:min_stock` | Crear |
| PUT | `/part/:id/:price` | Actualizar precio |
| DELETE | `/part/:id` | Eliminar |
| GET | `/partsLowStock` | Stock bajo |
| GET | `/partsSearch?q=texto` | Buscar |

### Motos
| Método | Endpoint | Descripción |
|--------|----------|-------------|
| GET | `/motos` | Listar todas |
| GET | `/moto/:id` | Obtener una |
| POST | `/moto/:plate/:brand/:model/:year/:owner_name/:owner_phone` | Crear |
| PUT | `/moto/:id/:owner_name/:owner_phone` | Actualizar dueño |
| DELETE | `/moto/:id` | Eliminar |
| GET | `/motoOrders/:id` | Historial de órdenes |

### Órdenes de Trabajo
| Método | Endpoint | Descripción |
|--------|----------|-------------|
| GET | `/orders` | Listar todas |
| GET | `/order/:id` | Obtener una |
| POST | `/order/:moto_id/:description` | Crear |
| PUT | `/orderStatus/:id/:status` | Cambiar estado |
| DELETE | `/order/:id` | Eliminar |
| GET | `/orderItems/:id` | Ver items |
| POST | `/orderItem/:id/:part_id/:quantity` | Agregar item (descuenta stock) |
| GET | `/orderTotal/:id` | Calcular total |

### Reportes
| Método | Endpoint | Descripción |
|--------|----------|-------------|
| GET | `/reportsInventory` | Valor total del inventario |
| GET | `/reportsLowStock` | Repuestos a reponer |
| GET | `/reportsMonthly` | Resumen del mes |

## Ejemplos

### Crear orden de trabajo
```bash
curl -X POST "http://localhost:8081/order/1/Service%2010000km"
```

### Agregar repuesto a orden (descuenta stock)
```bash
curl -X POST "http://localhost:8081/orderItem/1/1/2"  # orden 1, part 1, qty 2
```

### Ver total de orden
```bash
curl http://localhost:8081/orderTotal/1
```

### Cambiar estado de orden
```bash
curl -X PUT "http://localhost:8081/orderStatus/1/completed"
```

### Ver inventario total
```bash
curl http://localhost:8081/reportsInventory
# {"total_parts":4,"total_units":36,"total_value":783.5}
```

## Reglas de Negocio

1. **Stock automático**: Al agregar item a orden, se descuenta del stock
2. **Alerta stock bajo**: Cuando `stock < min_stock`, aparece en reportes
3. **Historial de precios**: Se guarda el precio al momento de usar el repuesto
4. **Estados de orden**: `pending` → `in_progress` → `completed`

## Estructura de Datos

### Part (Repuesto)
| Campo | Tipo | Descripción |
|-------|------|-------------|
| id | INTEGER | ID único |
| code | TEXT | Código interno |
| name | TEXT | Nombre |
| brand | TEXT | Marca |
| price | REAL | Precio |
| stock | INTEGER | Cantidad |
| min_stock | INTEGER | Stock mínimo |

### Moto
| Campo | Tipo | Descripción |
|-------|------|-------------|
| id | INTEGER | ID único |
| plate | TEXT | Patente |
| brand | TEXT | Marca |
| model | TEXT | Modelo |
| year | INTEGER | Año |
| owner_name | TEXT | Nombre dueño |
| owner_phone | TEXT | Teléfono |

### Order (Orden de Trabajo)
| Campo | Tipo | Descripción |
|-------|------|-------------|
| id | INTEGER | ID único |
| moto_id | INTEGER | ID de la moto |
| description | TEXT | Descripción |
| status | TEXT | Estado |
| created_at | TEXT | Fecha creación |

### OrderItem (Item de Orden)
| Campo | Tipo | Descripción |
|-------|------|-------------|
| id | INTEGER | ID único |
| order_id | INTEGER | ID de la orden |
| part_id | INTEGER | ID del repuesto |
| quantity | INTEGER | Cantidad |
| unit_price | REAL | Precio al momento |

## Datos de Prueba

**Repuestos:**
- ACE-001: Aceite 10W40 1L (Motul) - stock: 20
- FIL-001: Filtro de aceite (Honda) - stock: 8
- BUJ-001: Bujía NGK (NGK) - stock: 5 (bajo!)
- CAD-001: Cadena 428 (DID) - stock: 3

**Motos:**
- AB123CD: Honda CG 150 (2020) - Juan Perez
- XY789ZW: Yamaha YBR 125 (2019) - Maria Garcia
