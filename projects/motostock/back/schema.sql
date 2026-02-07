-- MotoStock - Schema de Base de Datos

-- Repuestos
CREATE TABLE IF NOT EXISTS parts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    code TEXT UNIQUE NOT NULL,
    name TEXT NOT NULL,
    brand TEXT,
    price REAL DEFAULT 0,
    stock INTEGER DEFAULT 0,
    min_stock INTEGER DEFAULT 5
);

-- Motos
CREATE TABLE IF NOT EXISTS motos (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    plate TEXT UNIQUE NOT NULL,
    brand TEXT NOT NULL,
    model TEXT NOT NULL,
    year INTEGER,
    owner_name TEXT NOT NULL,
    owner_phone TEXT
);

-- Órdenes de Trabajo
CREATE TABLE IF NOT EXISTS orders (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    moto_id INTEGER NOT NULL,
    description TEXT,
    status TEXT DEFAULT 'pending',
    created_at TEXT DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (moto_id) REFERENCES motos(id)
);

-- Items de Orden
CREATE TABLE IF NOT EXISTS order_items (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    order_id INTEGER NOT NULL,
    part_id INTEGER NOT NULL,
    quantity INTEGER DEFAULT 1,
    unit_price REAL,
    FOREIGN KEY (order_id) REFERENCES orders(id),
    FOREIGN KEY (part_id) REFERENCES parts(id)
);

-- Datos de prueba: Repuestos
INSERT OR IGNORE INTO parts (code, name, brand, price, stock, min_stock) VALUES
('ACE-001', 'Aceite 10W40 1L', 'Motul', 25.50, 20, 5),
('FIL-001', 'Filtro de aceite', 'Honda', 12.00, 8, 3),
('BUJ-001', 'Bujía NGK', 'NGK', 8.50, 5, 10),
('CAD-001', 'Cadena 428', 'DID', 45.00, 3, 2);

-- Datos de prueba: Motos
INSERT OR IGNORE INTO motos (plate, brand, model, year, owner_name, owner_phone) VALUES
('AB123CD', 'Honda', 'CG 150', 2020, 'Juan Perez', '1155551234'),
('XY789ZW', 'Yamaha', 'YBR 125', 2019, 'Maria Garcia', '1155555678');
