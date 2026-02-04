pub mod tokens;

use logos::Logos;
use serde::{Deserialize, Serialize};

pub use tokens::{Span, Spanned, Token};

/// Error de lexer
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LexError {
    pub message: String,
    pub span: Span,
}

/// Resultado del lexer: tokens con posiciones o errores
pub type LexResult = Result<Vec<Spanned<Token>>, Vec<LexError>>;

/// Tokeniza código fuente AURA
///
/// Retorna una lista de tokens con sus posiciones,
/// o una lista de errores si hay tokens inválidos.
pub fn tokenize(source: &str) -> LexResult {
    let mut tokens = Vec::new();
    let mut errors = Vec::new();

    let mut lexer = Token::lexer(source);

    while let Some(result) = lexer.next() {
        let span = Span::new(lexer.span().start, lexer.span().end);

        match result {
            Ok(token) => {
                // Ignorar comentarios que no son #test o #doc
                if let Token::Comment(ref c) = token {
                    if !c.starts_with("#test") && !c.starts_with("#doc") {
                        continue;
                    }
                }
                tokens.push(Spanned::new(token, span));
            }
            Err(_) => {
                errors.push(LexError {
                    message: format!("Token inválido: '{}'", lexer.slice()),
                    span,
                });
            }
        }
    }

    if errors.is_empty() {
        Ok(tokens)
    } else {
        Err(errors)
    }
}

/// Tokeniza y retorna errores en formato JSON (para agentes)
pub fn tokenize_json(source: &str) -> String {
    match tokenize(source) {
        Ok(tokens) => serde_json::to_string_pretty(&tokens).unwrap_or_default(),
        Err(errors) => serde_json::to_string_pretty(&errors).unwrap_or_default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capability() {
        let tokens = tokenize("+http").unwrap();
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].value, Token::Plus);
        assert!(matches!(tokens[1].value, Token::Ident(ref s) if s == "http"));
    }

    #[test]
    fn test_type_def() {
        let tokens = tokenize("@User {id:i name:s}").unwrap();
        assert!(tokens.len() > 0);
        assert_eq!(tokens[0].value, Token::At);
        assert!(matches!(tokens[1].value, Token::Ident(ref s) if s == "User"));
    }

    #[test]
    fn test_function() {
        let tokens = tokenize("add(a b) = a + b").unwrap();
        assert!(tokens.len() > 0);
        assert!(matches!(tokens[0].value, Token::Ident(ref s) if s == "add"));
    }

    #[test]
    fn test_effect_marker() {
        let tokens = tokenize("fetch!(url)").unwrap();
        assert!(tokens.iter().any(|t| t.value == Token::Bang));
    }

    #[test]
    fn test_string_interpolation() {
        let tokens = tokenize(r#""Hello {name}""#).unwrap();
        assert!(matches!(tokens[0].value, Token::String(ref s) if s == "Hello {name}"));
    }

    #[test]
    fn test_api_route() {
        let tokens = tokenize("GET /users/:id").unwrap();
        assert_eq!(tokens[0].value, Token::HttpGet);
        assert!(matches!(tokens[1].value, Token::Path(ref s) if s == "/users/:id"));
    }

    #[test]
    fn test_annotations() {
        let tokens = tokenize("@pk @unique @email").unwrap();
        assert_eq!(tokens[0].value, Token::AnnPk);
        assert_eq!(tokens[1].value, Token::AnnUnique);
        assert_eq!(tokens[2].value, Token::AnnEmail);
    }

    #[test]
    fn test_pattern_matching() {
        let tokens = tokenize("x | Ok(v) -> v | Err(e) -> nil").unwrap();
        assert!(tokens.iter().filter(|t| t.value == Token::Pipe).count() == 2);
        assert!(tokens.iter().filter(|t| t.value == Token::Arrow).count() == 2);
    }

    #[test]
    fn test_nullable() {
        let tokens = tokenize("email:s?").unwrap();
        assert!(tokens.iter().any(|t| t.value == Token::Question));
    }

    #[test]
    fn test_complete_example() {
        let source = r#"
+http +json

@User {
    id:uuid @pk
    name:s @min(2)
    email:s? @email
}

fetch(id) = http.get("users/{id}").json(User)
"#;
        let result = tokenize(source);
        assert!(result.is_ok(), "Failed to tokenize: {:?}", result);
    }
}
