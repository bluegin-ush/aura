# AURA - Agent-Unified Runtime Architecture

## Reglas para Claude

- **No hacer autoreferencias**: No incluir menciones a Claude en commits, comentarios de código, ni Co-Authored-By. Los commits deben ser neutrales.

## Proyecto

AURA es un lenguaje de programación diseñado para agentes IA, con sintaxis mínima para reducir tokens.

### Comandos útiles

```bash
# Build
cargo build --release

# Run
./target/release/aura run archivo.aura

# Tests
cargo test

# REPL
./target/release/aura repl
```

### Estructura

- `src/lexer/` - Tokenización
- `src/parser/` - Parsing a AST
- `src/vm/` - Máquina virtual / intérprete
- `src/caps/` - Capacidades (+http, +json, etc.)
