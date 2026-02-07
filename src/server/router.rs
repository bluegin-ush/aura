// Router para AURA
// Maneja rutas y extracción de parámetros

use std::collections::HashMap;

/// Una ruta definida en AURA
#[derive(Debug, Clone)]
pub struct Route {
    pub method: String,
    pub path: String,
    pub handler_name: String,
    pub param_names: Vec<String>,
}

impl Route {
    pub fn new(method: &str, path: &str, handler_name: &str) -> Self {
        let param_names = extract_param_names(path);
        Self {
            method: method.to_uppercase(),
            path: path.to_string(),
            handler_name: handler_name.to_string(),
            param_names,
        }
    }

    /// Verifica si esta ruta coincide con el método y path dados
    /// Retorna los parámetros extraídos si hay match
    pub fn matches(&self, method: &str, path: &str) -> Option<HashMap<String, String>> {
        if self.method != method.to_uppercase() {
            return None;
        }

        let route_parts: Vec<&str> = self.path.split('/').filter(|s| !s.is_empty()).collect();
        let path_parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

        if route_parts.len() != path_parts.len() {
            return None;
        }

        let mut params = HashMap::new();

        for (route_part, path_part) in route_parts.iter().zip(path_parts.iter()) {
            if route_part.starts_with(':') {
                // Es un parámetro
                let param_name = &route_part[1..];
                params.insert(param_name.to_string(), path_part.to_string());
            } else if route_part != path_part {
                // No coincide
                return None;
            }
        }

        Some(params)
    }
}

/// Extrae los nombres de parámetros de una ruta
/// "/users/:id/posts/:post_id" -> ["id", "post_id"]
fn extract_param_names(path: &str) -> Vec<String> {
    path.split('/')
        .filter(|s| s.starts_with(':'))
        .map(|s| s[1..].to_string())
        .collect()
}

/// Router que contiene todas las rutas
#[derive(Debug, Default)]
pub struct Router {
    routes: Vec<Route>,
}

impl Router {
    pub fn new() -> Self {
        Self { routes: Vec::new() }
    }

    pub fn add_route(&mut self, route: Route) {
        self.routes.push(route);
    }

    /// Encuentra la ruta que coincide con el método y path
    pub fn find_route(&self, method: &str, path: &str) -> Option<(&Route, HashMap<String, String>)> {
        for route in &self.routes {
            if let Some(params) = route.matches(method, path) {
                return Some((route, params));
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_route_matching() {
        let route = Route::new("GET", "/users/:id", "get_user");

        // Match exacto
        let params = route.matches("GET", "/users/123").unwrap();
        assert_eq!(params.get("id"), Some(&"123".to_string()));

        // Método diferente
        assert!(route.matches("POST", "/users/123").is_none());

        // Path diferente
        assert!(route.matches("GET", "/posts/123").is_none());
    }

    #[test]
    fn test_multiple_params() {
        let route = Route::new("GET", "/users/:user_id/posts/:post_id", "get_post");

        let params = route.matches("GET", "/users/1/posts/42").unwrap();
        assert_eq!(params.get("user_id"), Some(&"1".to_string()));
        assert_eq!(params.get("post_id"), Some(&"42".to_string()));
    }

    #[test]
    fn test_router() {
        let mut router = Router::new();
        router.add_route(Route::new("GET", "/users", "list_users"));
        router.add_route(Route::new("GET", "/users/:id", "get_user"));
        router.add_route(Route::new("POST", "/users", "create_user"));

        let (route, _) = router.find_route("GET", "/users").unwrap();
        assert_eq!(route.handler_name, "list_users");

        let (route, params) = router.find_route("GET", "/users/5").unwrap();
        assert_eq!(route.handler_name, "get_user");
        assert_eq!(params.get("id"), Some(&"5".to_string()));

        let (route, _) = router.find_route("POST", "/users").unwrap();
        assert_eq!(route.handler_name, "create_user");
    }
}
