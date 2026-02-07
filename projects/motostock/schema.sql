-- MotoStock - Schema de Base de Datos
-- Tabla de repuestos para taller de motos

CREATE TABLE IF NOT EXISTS parts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    code TEXT UNIQUE NOT NULL,
    name TEXT NOT NULL,
    brand TEXT,
    price REAL DEFAULT 0,
    stock INTEGER DEFAULT 0,
    min_stock INTEGER DEFAULT 5
);

-- Datos de prueba
INSERT INTO parts (code, name, brand, price, stock, min_stock) VALUES
('ACE-001', 'Aceite 10W40 1L', 'Motul', 25.50, 20, 5),
('FIL-001', 'Filtro de aceite', 'Honda', 12.00, 8, 3),
('BUJ-001', 'Bujía NGK', 'NGK', 8.50, 5, 10),
('CAD-001', 'Cadena 428', 'DID', 45.00, 3, 2);
