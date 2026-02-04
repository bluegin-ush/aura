# v2 - Especificación de Sintaxis

## Resumen

AURA usa una sintaxis minimalista optimizada para generación por agentes de IA.

## Elementos fundamentales

### Capacidades (reemplazan imports)

```ruby
+http                     # Habilita cliente HTTP
+json                     # Serialización JSON
+db                       # Base de datos
+auth                     # Autenticación/autorización
+ws                       # WebSockets
+fs                       # Sistema de archivos
+crypto                   # Criptografía
+valid                    # Validaciones adicionales
+time                     # Fecha/hora
+email                    # Envío de emails
```

Múltiples capacidades en una línea:
```ruby
+http +json +db +auth
```

### Tipos primitivos

| Símbolo | Tipo | Ejemplo |
|---------|------|---------|
| `i` | entero | `42` |
| `f` | flotante | `3.14` |
| `s` | string | `"hello"` |
| `b` | booleano | `true` / `false` |
| `ts` | timestamp | `now()` |
| `uuid` | UUID | `uuid()` |
| `?` | nullable | `s?` = string o nil |
| `[]` | lista | `[i]` = lista de enteros |
| `{}` | map/record | `{s:i}` = string→int |

### Definición de tipos (structs)

```ruby
@User {
  id:uuid
  name:s
  email:s?
  age:i
  active:b
}
```

Con anotaciones:
```ruby
@User {
  id:uuid @pk                    # Primary key
  email:s @unique @email         # Único, validar formato email
  pass:s @hash @hide             # Auto-hash, ocultar en JSON
  name:s @min(2) @max(100)       # Validación de longitud
  role:Role = .user              # Valor default
  created:ts @auto               # Auto-generado
}
```

### Anotaciones disponibles

| Anotación | Propósito |
|-----------|-----------|
| `@pk` | Primary key |
| `@unique` | Valor único |
| `@email` | Validar formato email |
| `@url` | Validar formato URL |
| `@min(n)` | Mínimo (longitud o valor) |
| `@max(n)` | Máximo (longitud o valor) |
| `@range(a,b)` | Rango de valores |
| `@match(regex)` | Validar con regex |
| `@hash` | Auto-hashear (passwords) |
| `@hide` | Ocultar en serialización |
| `@auto` | Auto-generar |
| `@rel` | Relación con otro tipo |
| `@index` | Crear índice DB |

### Enums

```ruby
@Status = pending | active | done | cancelled

@Role = user | admin | mod
```

Con valores asociados:
```ruby
@Result = Ok(v) | Err(s)
```

### Funciones

Sintaxis básica:
```ruby
nombre(args) = expresión
```

Ejemplos:
```ruby
# Función simple
add(a b) = a + b

# Con tipos explícitos (opcional, se infieren)
add(a:i b:i) -> i = a + b

# Multilinea con :
process(data) = :
  validated = validate(data)
  transformed = transform(validated)
  save(transformed)
```

### Efectos (IO, async)

Las funciones con efectos se marcan con `!`:
```ruby
# Función pura (sin efectos)
double(x) = x * 2

# Función con efectos
fetch!(url) = http.get!(url)

# Llamar función con efectos requiere !
main! = :
  data = fetch!("api/users")
  print!(data)
```

### Pipes

El operador `|` encadena transformaciones:
```ruby
process(ids) = ids | map(fetch) | filter(_.active) | sort(_.name)
```

Equivalente a:
```ruby
process(ids) = sort(filter(map(ids, fetch), _.active), _.name)
```

### Pattern matching

Inline con `|`:
```ruby
handle(r) = r | Ok(v) -> process(v) | Err(e) -> log(e)
```

Multilinea:
```ruby
handle(r) = r |
  Ok(v) -> process(v)
  Err(e) -> log(e)
  nil -> default_value
```

### Placeholder `_`

El guión bajo representa el argumento implícito:
```ruby
users | filter(_.active)          # _.active = u -> u.active
users | map(_.name)               # _.name = u -> u.name
users | sort(_.age)               # _.age = u -> u.age
```

### Acceso a propiedades

```ruby
user.name                         # Acceso directo
user.address?.city                # Safe navigation (nil si address es nil)
user["name"]                      # Acceso dinámico
```

### Operadores

```ruby
# Aritméticos
+ - * / %

# Comparación
== != < > <= >=

# Lógicos
& (and) | (or) ! (not)

# String
++ (concatenación)

# Null
?? (null coalescing): x ?? default
?. (safe navigation): x?.y
```

### Interpolación de strings

```ruby
"Hello {name}"
"User {user.id}: {user.name}"
"Total: {items | len}"
```

### Listas

```ruby
[]                                # Lista vacía
[1 2 3]                          # Lista de enteros (sin comas)
[1, 2, 3]                        # Comas opcionales
users | first                     # Primer elemento
users | last                      # Último elemento
users | len                       # Longitud
users | get(0)                    # Por índice
users | slice(0 10)              # Sublista
```

### Maps/Records

```ruby
{}                                # Map vacío
{name:"John" age:30}             # Record (sin comas)
{name: "John", age: 30}          # Comas opcionales
data.name                         # Acceso
data | keys                       # Lista de keys
data | values                     # Lista de values
{...base extra:"field"}          # Spread
```

### Comentarios

```ruby
# Comentario de línea

#doc función: "Descripción de la función"

#test nombre: expresión_booleana
```

### Tests inline

```ruby
#test user_exists: fetch(1).name == "John"
#test user_missing: fetch(999) == nil
#test add_works: add(2 3) == 5
```

### Documentación inline

```ruby
#doc fetch: "Obtiene usuario por ID, retorna nil si no existe"
#doc @User: "Representa un usuario del sistema"
```

## Ejemplos completos

### CRUD básico

```ruby
+http +db +auth

@User {
  id:uuid @pk
  email:s @unique @email
  name:s @min(2)
  created:ts @auto
}

@User +crud                      # Genera: create! get! list! update! delete!
```

### API REST

```ruby
+http +db +auth

@Post {
  id:uuid @pk
  author:User @rel
  title:s @min(5) @max(200)
  body:s
  published:b = false
}

+api("/v1"):
  POST /signup        -> signup!
  POST /login         -> login!

  @auth:
    GET  /me          -> @me

    /posts:
      GET  /          -> Post.list!(author:@me)
      POST /          -> Post.create!({..@body author:@me})
      GET  /:id       -> Post.get!(@id)
      PUT  /:id       -> Post.update!(@id @body) @own
      DEL  /:id       -> Post.delete!(@id) @own
```

### WebSockets

```ruby
+ws("/live"):
  @auth:
    sub posts:new     -> Post.stream!(published:true)
    sub posts/:id     -> Post.changes!(@id)
    pub posts/:id/like-> Post.update!(@id {likes:+1})
```

### Background jobs

```ruby
+job send_welcome @on(User.created):
  u -> email.send!(u.email "welcome" tpl:welcome_email)

+job cleanup @every(1.day):
  Post.delete!(published:false created < now()-30.days)
```

## Formato canónico

No existe formateo opcional. Esta es la única forma válida:

```ruby
# VÁLIDO
fetch(id) = http.get("users/{id}")

# INVÁLIDO (espacios extra)
fetch( id ) = http.get( "users/{id}" )

# INVÁLIDO (líneas extra innecesarias)
fetch(id) =
  http.get("users/{id}")
```

El parser rechaza código que no siga el formato canónico.

## Palabras reservadas

```
true false nil
if else match
for in while
return break continue
use
```

## Gramática EBNF (simplificada)

```ebnf
program     = capability* definition*
capability  = '+' IDENT
definition  = type_def | func_def | enum_def | api_def | test_def
type_def    = '@' IDENT '{' field* '}' annotation*
field       = IDENT ':' type annotation* ('=' expr)?
type        = primitive | IDENT | '[' type ']' | '{' type ':' type '}'
primitive   = 'i' | 'f' | 's' | 'b' | 'ts' | 'uuid'
func_def    = IDENT '!'? '(' params ')' ('->' type)? '=' expr
expr        = literal | IDENT | call | pipe | match | block
pipe        = expr '|' expr
call        = expr '(' args ')'
block       = ':' NEWLINE INDENT expr+ DEDENT
```

## Próximo

v3-runtime.md: Especificación del runtime y ejecución.
