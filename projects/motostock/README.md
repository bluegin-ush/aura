# MotoStock - API de Gestión de Stock

API REST para gestión de inventario de taller de motos, construida con AURA.

## Requisitos

- AURA compilado con soporte para `aura serve`
- curl (para tests)

## Inicio Rápido

```bash
# Ir al directorio del proyecto
cd projects/motostock

# Inicializar la base de datos
aura run init.aura

# Iniciar el servidor
aura serve motostock.aura --port 8081

# En otra terminal, ejecutar tests
./test_api.sh
```

## Endpoints

| Método | Endpoint | Descripción |
|--------|----------|-------------|
| GET | `/health` | Estado del servidor |
| GET | `/parts` | Listar todos los repuestos |
| GET | `/part/:id` | Obtener un repuesto |
| POST | `/part/:code/:name/:brand/:price/:stock/:min_stock` | Crear repuesto |
| PUT | `/part/:id/:price` | Actualizar precio |
| DELETE | `/part/:id` | Eliminar repuesto |
| GET | `/partsLowStock` | Repuestos con stock bajo |
| GET | `/partsSearch?q=texto` | Buscar repuestos |

## Ejemplos

### Listar repuestos
```bash
curl http://localhost:8081/parts
```

### Obtener un repuesto
```bash
curl http://localhost:8081/part/1
```

### Crear repuesto
```bash
curl -X POST "http://localhost:8081/part/PIS-001/Piston%20150cc/Honda/85.00/4/2"
```

### Actualizar precio
```bash
curl -X PUT "http://localhost:8081/part/1/30.00"
```

### Eliminar repuesto
```bash
curl -X DELETE http://localhost:8081/part/5
```

### Buscar repuestos
```bash
curl "http://localhost:8081/partsSearch?q=aceite"
```

### Ver stock bajo
```bash
curl http://localhost:8081/partsLowStock
```

## Estructura de Datos

### Part (Repuesto)

| Campo | Tipo | Descripción |
|-------|------|-------------|
| id | INTEGER | ID autoincremental |
| code | TEXT | Código único del repuesto |
| name | TEXT | Nombre del repuesto |
| brand | TEXT | Marca |
| price | REAL | Precio |
| stock | INTEGER | Cantidad en stock |
| min_stock | INTEGER | Stock mínimo (para alertas) |

## Archivos

| Archivo | Descripción |
|---------|-------------|
| `motostock.aura` | API principal |
| `init.aura` | Script de inicialización de BD |
| `schema.sql` | Schema SQL de referencia |
| `test_api.sh` | Tests automatizados |

## Base de Datos

La API usa SQLite. La base de datos se crea ejecutando `init.aura` con datos de prueba:

- ACE-001: Aceite 10W40 1L (Motul) - stock: 20
- FIL-001: Filtro de aceite (Honda) - stock: 8
- BUJ-001: Bujía NGK (NGK) - stock: 5 (bajo stock!)
- CAD-001: Cadena 428 (DID) - stock: 3
