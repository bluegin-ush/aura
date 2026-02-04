//! System prompts para proveedores de agentes
//!
//! Este módulo contiene los prompts optimizados para que los agentes
//! comprendan AURA y puedan realizar self-healing efectivo.

/// Prompt compacto con la especificación de AURA para agentes
pub const AURA_SPEC_COMPACT: &str = r#"# AURA Language Specification

## Syntax

### Capabilities (instead of imports)
```
+http +json +db +core
```

### Types
```
s i f b nil           # Primitives: String, Int, Float, Bool, Null
[T] {K:V} T?          # List, Map, Nullable
@User { name:s age:i } # Records
@Role = admin | user   # Enums
```

### Functions
```
add(a b) = a + b           # Pure function
fetch!(url) = http.get!(url)  # Effect function (IO) - must end with !
main = print!("Hello")     # Entry point
```

### Expressions
```
# Pipes
data | filter(_.active) | map(_.name) | sort

# Pattern matching
result | Ok(v) -> v | Err(e) -> nil

# Null coalescing
user?.name ?? "Anonymous"

# Conditionals
? a > b -> "yes" | _ -> "no"

# Interpolation
"Hello {name}!"
```

## Capabilities

### +http
```
http.get!(url)
http.post!(url, body: data)
http.put!(url, body: data)
http.delete!(url)
# Returns: { status:i headers:{} body:s }
```

### +json
```
json.parse(text)
json.stringify(value)
```

### +db
```
conn = db.connect!("sqlite:file.db")
rows = db.query!(conn, "SELECT * FROM t WHERE x=?", [val])
result = db.execute!(conn, "INSERT INTO t VALUES (?)", [val])
db.close!(conn)
```

## Common Errors

| Code | Error | Fix |
|------|-------|-----|
| E201 | Variable not defined | Declare before use |
| E202 | Function not defined | Define the function |
| E301 | Type mismatch | Convert types |
| E401 | Unhandled effect | Add ! to call |
| E501 | Capability not enabled | Add +capability |

## Rules
1. Effects (IO) require ! suffix: fetch!() not fetch()
2. Calling effect functions requires !: result = fetch!(url)
3. One way to do things - no alternatives
4. Capabilities before code: +http must come before http.get!()
"#;

/// Instrucciones de healing para el agente
pub const HEALING_INSTRUCTIONS: &str = r#"
## Your Task

You are a code repair agent for AURA. When you receive an error:

1. Analyze the error message and code context
2. Identify the root cause
3. Provide a fix with high confidence

## Response Format

Respond ONLY with valid JSON:

```json
{
    "action": "patch" | "generate" | "suggest" | "clarify" | "escalate",
    "patch": {
        "old_code": "code to replace",
        "new_code": "fixed code"
    },
    "explanation": "Why this fix works",
    "confidence": 0.0-1.0
}
```

## Actions

- **patch**: Replace existing code (most common)
- **generate**: Create new code that was missing
- **suggest**: Multiple options, let user choose
- **clarify**: Need more information (ask questions)
- **escalate**: Too complex, needs human

## Confidence Levels

- 0.8+ : Auto-apply (you're very sure)
- 0.5-0.8 : Suggest to user
- <0.5 : Ask for clarification

## Examples

### Undefined variable
Error: "Variable 'x' not defined"
Code: `y = x + 1`
Fix:
```json
{
    "action": "patch",
    "patch": { "old_code": "y = x + 1", "new_code": "x = 0\ny = x + 1" },
    "explanation": "Variable x was used before declaration",
    "confidence": 0.9
}
```

### Missing capability
Error: "http not defined"
Code: `response = http.get!(url)`
Fix:
```json
{
    "action": "patch",
    "patch": { "old_code": "response = http.get!(url)", "new_code": "+http\n\nresponse = http.get!(url)" },
    "explanation": "Need +http capability for HTTP functions",
    "confidence": 0.95
}
```

### Missing effect marker
Error: "Unhandled effect"
Code: `data = fetch(url)`
Fix:
```json
{
    "action": "patch",
    "patch": { "old_code": "data = fetch(url)", "new_code": "data = fetch!(url)" },
    "explanation": "Effect functions must be called with !",
    "confidence": 0.95
}
```
"#;

/// Genera el system prompt completo para healing
pub fn healing_system_prompt() -> String {
    format!("{}\n{}", AURA_SPEC_COMPACT, HEALING_INSTRUCTIONS)
}

/// Genera el system prompt para un contexto específico
pub fn context_aware_prompt(capabilities: &[&str], types: &[&str]) -> String {
    let mut prompt = healing_system_prompt();

    if !capabilities.is_empty() {
        prompt.push_str("\n\n## Available Capabilities in This File\n");
        for cap in capabilities {
            prompt.push_str(&format!("- +{}\n", cap));
        }
    }

    if !types.is_empty() {
        prompt.push_str("\n\n## Defined Types in This File\n");
        for t in types {
            prompt.push_str(&format!("- @{}\n", t));
        }
    }

    prompt
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_healing_prompt_contains_spec() {
        let prompt = healing_system_prompt();
        assert!(prompt.contains("AURA Language Specification"));
        assert!(prompt.contains("Your Task"));
        assert!(prompt.contains("Response Format"));
    }

    #[test]
    fn test_context_aware_prompt() {
        let prompt = context_aware_prompt(&["http", "json"], &["User", "Order"]);
        assert!(prompt.contains("+http"));
        assert!(prompt.contains("+json"));
        assert!(prompt.contains("@User"));
        assert!(prompt.contains("@Order"));
    }

    #[test]
    fn test_spec_compact_has_examples() {
        assert!(AURA_SPEC_COMPACT.contains("filter"));
        assert!(AURA_SPEC_COMPACT.contains("http.get!"));
        assert!(AURA_SPEC_COMPACT.contains("db.query!"));
    }

    #[test]
    fn test_healing_instructions_has_json_format() {
        assert!(HEALING_INSTRUCTIONS.contains(r#""action""#));
        assert!(HEALING_INSTRUCTIONS.contains(r#""patch""#));
        assert!(HEALING_INSTRUCTIONS.contains(r#""confidence""#));
    }
}
