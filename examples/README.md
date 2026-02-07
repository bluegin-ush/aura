# Ejemplos AURA vs Python

Comparación de escenarios reales mostrando la reducción de tokens y complejidad.

---

## 1. API Client

### AURA (4 líneas, ~45 tokens)

```ruby
+http +json

get_user(id) = : url = "https://jsonplaceholder.typicode.com/users/{id}"; response = http.get(url); json.parse(response.body)
format_user(user) = "User: {user.name} - {user.email}"
main = : user = get_user(1); format_user(user)
```

```
$ aura run 01_api_client.aura
User: Leanne Graham - Sincere@april.biz
```

### Python equivalente (~25 líneas, ~180 tokens)

```python
import requests
from typing import Optional
from dataclasses import dataclass

@dataclass
class User:
    id: int
    name: str
    email: str
    # ... más campos

def get_user(user_id: int) -> Optional[User]:
    url = f"https://jsonplaceholder.typicode.com/users/{user_id}"
    response = requests.get(url)
    if response.status_code == 200:
        data = response.json()
        return User(**data)
    return None

def format_user(user: User) -> str:
    return f"User: {user.name} - {user.email}"

if __name__ == "__main__":
    user = get_user(1)
    if user:
        print(format_user(user))
```

### Reducción: 75% menos código, 75% menos tokens

---

## 2. CRUD con Base de Datos

### AURA (8 líneas, ~120 tokens)

```ruby
+db +json

init_db(conn) = db.execute(conn, "CREATE TABLE IF NOT EXISTS users (id INTEGER PRIMARY KEY, name TEXT, email TEXT, active INTEGER)")
create_user(conn, name, email) = db.execute(conn, "INSERT INTO users (name, email, active) VALUES (?, ?, 1)", [name, email])
get_users(conn) = db.query(conn, "SELECT * FROM users", [])
update_user(conn, id, name, email) = db.execute(conn, "UPDATE users SET name = ?, email = ? WHERE id = ?", [name, email, id])
delete_user(conn, id) = db.execute(conn, "DELETE FROM users WHERE id = ?", [id])

main = : conn = db.connect("sqlite::memory:"); init_db(conn); create_user(conn, "Alice", "alice@example.com"); create_user(conn, "Bob", "bob@example.com"); users = get_users(conn); db.close(conn); users
```

```
$ aura run 02_crud_users.aura
[{name:Alice active:1 email:alice@example.com id:1} {id:2 name:Bob email:bob@example.com active:1}]
```

### Python equivalente (~65 líneas, ~450 tokens)

```python
import sqlite3
from dataclasses import dataclass
from typing import List, Optional

@dataclass
class User:
    id: Optional[int]
    name: str
    email: str
    active: bool = True

class UserRepository:
    def __init__(self, db_path: str = ":memory:"):
        self.conn = sqlite3.connect(db_path)
        self.conn.row_factory = sqlite3.Row
        self._init_db()

    def _init_db(self):
        self.conn.execute("""
            CREATE TABLE IF NOT EXISTS users (
                id INTEGER PRIMARY KEY,
                name TEXT,
                email TEXT,
                active INTEGER
            )
        """)
        self.conn.commit()

    def create(self, name: str, email: str) -> int:
        cursor = self.conn.execute(
            "INSERT INTO users (name, email, active) VALUES (?, ?, 1)",
            (name, email)
        )
        self.conn.commit()
        return cursor.lastrowid

    def get_all(self) -> List[User]:
        cursor = self.conn.execute("SELECT * FROM users")
        return [User(**dict(row)) for row in cursor.fetchall()]

    def update(self, id: int, name: str, email: str):
        self.conn.execute(
            "UPDATE users SET name = ?, email = ? WHERE id = ?",
            (name, email, id)
        )
        self.conn.commit()

    def delete(self, id: int):
        self.conn.execute("DELETE FROM users WHERE id = ?", (id,))
        self.conn.commit()

    def close(self):
        self.conn.close()

if __name__ == "__main__":
    repo = UserRepository()
    repo.create("Alice", "alice@example.com")
    repo.create("Bob", "bob@example.com")
    users = repo.get_all()
    repo.close()
    print(users)
```

### Reducción: 87% menos código, 73% menos tokens

---

## 3. Análisis de Datos

### AURA (4 líneas, ~60 tokens)

```ruby
+http +json

fetch_posts = : r = http.get("https://jsonplaceholder.typicode.com/posts"); json.parse(r.body)
fetch_users = : r = http.get("https://jsonplaceholder.typicode.com/users"); json.parse(r.body)
main = : posts = fetch_posts(); users = fetch_users(); tp = len(posts); tu = len(users); avg = tp / tu; "Reporte: {tp} posts, {tu} usuarios, promedio {avg} posts/usuario"
```

```
$ aura run 03_data_analysis.aura
Reporte: 100 posts, 10 usuarios, promedio 10 posts/usuario
```

### Python equivalente (~35 líneas, ~250 tokens)

```python
import requests
from typing import List, Dict, Any

def fetch_posts() -> List[Dict[str, Any]]:
    response = requests.get("https://jsonplaceholder.typicode.com/posts")
    response.raise_for_status()
    return response.json()

def fetch_users() -> List[Dict[str, Any]]:
    response = requests.get("https://jsonplaceholder.typicode.com/users")
    response.raise_for_status()
    return response.json()

def generate_report(posts: List, users: List) -> str:
    total_posts = len(posts)
    total_users = len(users)
    avg = total_posts / total_users if total_users > 0 else 0
    return f"Reporte: {total_posts} posts, {total_users} usuarios, promedio {avg} posts/usuario"

if __name__ == "__main__":
    posts = fetch_posts()
    users = fetch_users()
    print(generate_report(posts, users))
```

### Reducción: 88% menos código, 76% menos tokens

---

## Resumen

| Escenario | AURA | Python | Reducción Código | Reducción Tokens |
|-----------|------|--------|------------------|------------------|
| API Client | 4 líneas | 25 líneas | 84% | 75% |
| CRUD | 8 líneas | 65 líneas | 87% | 73% |
| Análisis | 4 líneas | 35 líneas | 88% | 76% |

**Promedio: 86% menos código, 75% menos tokens**

---

## Por qué importa para agentes IA

1. **Menos tokens = Menos costo**: Un agente que genera código AURA consume 75% menos tokens que uno que genera Python.

2. **Menos complejidad = Menos errores**: Menos líneas significa menos lugares donde equivocarse.

3. **Un archivo = Contexto completo**: El agente no necesita leer múltiples archivos para entender el código.

4. **Sin boilerplate = Foco en lógica**: El agente se concentra en el problema, no en imports y configuración.
