//! Formateo de errores con colores y contexto usando ariadne.
//!
//! Este modulo proporciona funciones para mostrar errores de AURA de forma
//! legible para humanos, con codigo fuente resaltado, numeros de linea,
//! y sugerencias de correccion.

use ariadne::{Color, Config, Label, Report, ReportKind, Source};
use std::fmt::Write as FmtWrite;

use crate::error::{AuraError, Severity};
use crate::lexer::Span;

/// Tipo de error para categorizar el formateo
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorType {
    /// Errores de sintaxis (E0xx)
    Parse,
    /// Errores de tipos (E1xx)
    Type,
    /// Errores de referencias (E2xx)
    Reference,
    /// Errores de efectos (E3xx)
    Effect,
    /// Errores de runtime (E4xx)
    Runtime,
    /// Errores de capacidades (E5xx)
    Capability,
    /// Errores de agente (E9xx)
    Agent,
}

impl ErrorType {
    /// Determina el tipo de error basado en el codigo de error
    pub fn from_code(code: &str) -> Self {
        if code.starts_with("E0") {
            ErrorType::Parse
        } else if code.starts_with("E1") {
            ErrorType::Type
        } else if code.starts_with("E2") {
            ErrorType::Reference
        } else if code.starts_with("E3") {
            ErrorType::Effect
        } else if code.starts_with("E4") {
            ErrorType::Runtime
        } else if code.starts_with("E5") {
            ErrorType::Capability
        } else if code.starts_with("E9") {
            ErrorType::Agent
        } else {
            ErrorType::Runtime
        }
    }

    /// Color primario para este tipo de error
    fn primary_color(&self) -> Color {
        match self {
            ErrorType::Parse => Color::Red,
            ErrorType::Type => Color::Magenta,
            ErrorType::Reference => Color::Yellow,
            ErrorType::Effect => Color::Cyan,
            ErrorType::Runtime => Color::Red,
            ErrorType::Capability => Color::Blue,
            ErrorType::Agent => Color::Green,
        }
    }

    /// Nombre legible del tipo de error
    fn display_name(&self) -> &'static str {
        match self {
            ErrorType::Parse => "error de sintaxis",
            ErrorType::Type => "error de tipo",
            ErrorType::Reference => "referencia invalida",
            ErrorType::Effect => "error de efecto",
            ErrorType::Runtime => "error de ejecucion",
            ErrorType::Capability => "capacidad faltante",
            ErrorType::Agent => "error de agente",
        }
    }
}

/// Convierte severidad de AURA a tipo de reporte de ariadne
fn severity_to_report_kind(severity: &Severity) -> ReportKind<'static> {
    match severity {
        Severity::Error => ReportKind::Error,
        Severity::Warning => ReportKind::Warning,
        Severity::Info => ReportKind::Advice,
    }
}

/// Formatea un error de AURA de forma bonita con colores y contexto.
///
/// # Argumentos
///
/// * `error` - El error de AURA a formatear
/// * `source` - El codigo fuente completo
/// * `filename` - Nombre del archivo para mostrar en el encabezado
///
/// # Retorna
///
/// Una cadena con el error formateado, incluyendo:
/// - Mensaje de error con codigo
/// - Fragmento de codigo con numeros de linea
/// - Indicador de la ubicacion exacta del error
/// - Sugerencia de correccion si esta disponible
///
/// # Ejemplo
///
/// ```
/// use aura::error::{AuraError, ErrorCode, Severity, Location};
/// use aura::error::pretty::format_error_pretty;
///
/// let error = AuraError::new(
///     ErrorCode::reference(1),
///     Severity::Error,
///     Location { file: "test.aura".into(), line: 2, col: 5, end_col: Some(15), span: None },
///     "Variable no definida: 'nombre'"
/// );
///
/// let source = "x = 1\ny = nombre + 1";
/// let output = format_error_pretty(&error, source, "test.aura");
/// println!("{}", output);
/// ```
pub fn format_error_pretty(error: &AuraError, source: &str, filename: &str) -> String {
    let mut output = Vec::new();

    let error_type = ErrorType::from_code(&error.code.0);
    let color = error_type.primary_color();

    // Calcular el span en bytes desde la ubicacion
    let span = location_to_span(&error.location, source);

    // Construir el reporte
    let mut report = Report::build(
        severity_to_report_kind(&error.severity),
        filename,
        span.start,
    )
    .with_code(&error.code.0)
    .with_message(&error.message)
    .with_config(Config::default().with_color(true));

    // Etiqueta principal en la ubicacion del error
    let label_message = format!("{}", error_type.display_name());
    report = report.with_label(
        Label::new((filename, span.start..span.end))
            .with_message(label_message)
            .with_color(color),
    );

    // Agregar sugerencia si existe
    if let Some(ref suggestion) = error.suggestion {
        let suggestion_text = if let Some(ref replacement) = suggestion.replacement {
            format!("{}: {}", suggestion.message, replacement)
        } else {
            suggestion.message.clone()
        };
        report = report.with_help(suggestion_text);
    }

    // Agregar detalles adicionales si existen
    if let Some(ref details) = error.details {
        if let Some(note) = details.get("note").and_then(|v| v.as_str()) {
            report = report.with_note(note);
        }
    }

    // Escribir el reporte a un buffer
    report
        .finish()
        .write((filename, Source::from(source)), &mut output)
        .expect("Error al escribir el reporte");

    String::from_utf8(output).unwrap_or_else(|_| error.message.clone())
}

/// Formatea multiples errores de AURA
///
/// # Argumentos
///
/// * `errors` - Lista de errores a formatear
/// * `source` - El codigo fuente completo
/// * `filename` - Nombre del archivo
///
/// # Retorna
///
/// Todos los errores formateados concatenados
pub fn format_errors_pretty(errors: &[AuraError], source: &str, filename: &str) -> String {
    let mut output = String::new();

    for (i, error) in errors.iter().enumerate() {
        if i > 0 {
            output.push('\n');
        }
        output.push_str(&format_error_pretty(error, source, filename));
    }

    // Resumen al final si hay multiples errores
    if errors.len() > 1 {
        let error_count = errors.iter().filter(|e| e.severity == Severity::Error).count();
        let warning_count = errors.iter().filter(|e| e.severity == Severity::Warning).count();

        writeln!(&mut output).ok();
        write!(
            &mut output,
            "Total: {} error(es), {} advertencia(s)",
            error_count, warning_count
        )
        .ok();
    }

    output
}

/// Convierte una ubicacion (linea, columna) a un span de bytes
fn location_to_span(location: &crate::error::Location, source: &str) -> Span {
    let mut current_line = 1;
    let mut line_start = 0;

    // Encontrar el inicio de la linea
    for (i, c) in source.char_indices() {
        if current_line == location.line {
            line_start = i;
            break;
        }
        if c == '\n' {
            current_line += 1;
        }
    }

    // Calcular posicion en bytes
    let start = line_start + location.col.saturating_sub(1);
    let end = if let Some(end_col) = location.end_col {
        line_start + end_col.saturating_sub(1)
    } else {
        // Si no hay end_col, marcar solo un caracter
        start + 1
    };

    // Asegurar que no excedemos el largo del source
    let start = start.min(source.len());
    let end = end.min(source.len()).max(start);

    Span::new(start, end)
}

/// Formatea un error de parse directamente desde un span
///
/// Util para errores del lexer y parser que ya tienen span
pub fn format_parse_error(
    message: &str,
    span: &Span,
    source: &str,
    filename: &str,
    suggestion: Option<&str>,
) -> String {
    let mut output = Vec::new();

    let mut report = Report::build(ReportKind::Error, filename, span.start)
        .with_code("E001")
        .with_message(message)
        .with_config(Config::default().with_color(true))
        .with_label(
            Label::new((filename, span.start..span.end))
                .with_message("aqui")
                .with_color(Color::Red),
        );

    if let Some(sugg) = suggestion {
        report = report.with_help(sugg);
    }

    report
        .finish()
        .write((filename, Source::from(source)), &mut output)
        .expect("Error al escribir el reporte");

    String::from_utf8(output).unwrap_or_else(|_| message.to_string())
}

/// Formatea un error de tipo con informacion adicional
pub fn format_type_error(
    message: &str,
    span: &Span,
    source: &str,
    filename: &str,
    expected: &str,
    found: &str,
) -> String {
    let mut output = Vec::new();

    let label_msg = format!("esperado '{}', encontrado '{}'", expected, found);

    Report::build(ReportKind::Error, filename, span.start)
        .with_code("E101")
        .with_message(message)
        .with_config(Config::default().with_color(true))
        .with_label(
            Label::new((filename, span.start..span.end))
                .with_message(label_msg)
                .with_color(Color::Magenta),
        )
        .with_help(format!("El tipo debe ser '{}'", expected))
        .finish()
        .write((filename, Source::from(source)), &mut output)
        .expect("Error al escribir el reporte");

    String::from_utf8(output).unwrap_or_else(|_| message.to_string())
}

/// Formatea un error de referencia (variable/funcion no definida)
pub fn format_reference_error(
    name: &str,
    span: &Span,
    source: &str,
    filename: &str,
    similar: Option<&[&str]>,
) -> String {
    let mut output = Vec::new();

    let message = format!("'{}' no esta definido", name);

    let mut report = Report::build(ReportKind::Error, filename, span.start)
        .with_code("E201")
        .with_message(&message)
        .with_config(Config::default().with_color(true))
        .with_label(
            Label::new((filename, span.start..span.end))
                .with_message("referencia no encontrada")
                .with_color(Color::Yellow),
        );

    // Sugerir nombres similares si los hay
    if let Some(similars) = similar {
        if !similars.is_empty() {
            let suggestions = similars.join(", ");
            report = report.with_help(format!("Quizas quisiste decir: {}", suggestions));
        }
    }

    report
        .finish()
        .write((filename, Source::from(source)), &mut output)
        .expect("Error al escribir el reporte");

    String::from_utf8(output).unwrap_or_else(|_| message)
}

/// Formatea un error de capacidad faltante
pub fn format_capability_error(
    capability: &str,
    span: &Span,
    source: &str,
    filename: &str,
) -> String {
    let mut output = Vec::new();

    let message = format!(
        "La capacidad '+{}' es requerida pero no esta declarada",
        capability
    );

    Report::build(ReportKind::Error, filename, span.start)
        .with_code("E501")
        .with_message(&message)
        .with_config(Config::default().with_color(true))
        .with_label(
            Label::new((filename, span.start..span.end))
                .with_message(format!("requiere +{}", capability))
                .with_color(Color::Blue),
        )
        .with_help(format!(
            "Agrega '+{}' al inicio del archivo",
            capability
        ))
        .finish()
        .write((filename, Source::from(source)), &mut output)
        .expect("Error al escribir el reporte");

    String::from_utf8(output).unwrap_or_else(|_| message)
}

/// Formatea un error de efecto no manejado
pub fn format_effect_error(
    function: &str,
    span: &Span,
    source: &str,
    filename: &str,
) -> String {
    let mut output = Vec::new();

    let message = format!(
        "La funcion '{}' tiene efectos pero no esta marcada con '!'",
        function
    );

    Report::build(ReportKind::Error, filename, span.start)
        .with_code("E301")
        .with_message(&message)
        .with_config(Config::default().with_color(true))
        .with_label(
            Label::new((filename, span.start..span.end))
                .with_message("funcion con efectos")
                .with_color(Color::Cyan),
        )
        .with_help(format!(
            "Usa '{}!(...)' para indicar que la funcion tiene efectos",
            function
        ))
        .finish()
        .write((filename, Source::from(source)), &mut output)
        .expect("Error al escribir el reporte");

    String::from_utf8(output).unwrap_or_else(|_| message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::{ErrorCode, Location};

    fn create_test_source() -> &'static str {
        r#"+http +json

@User {
    id:uuid @pk
    name:s @min(2)
    email:s? @email
}

users = fetch_users!()
report = generate_report(users)
send_email!(report)"#
    }

    #[test]
    fn test_format_parse_error() {
        let source = "x = 1 +\ny = 2";
        let span = Span::new(6, 7);

        let output = format_parse_error(
            "Expresion incompleta",
            &span,
            source,
            "test.aura",
            Some("Agrega una expresion despues del operador"),
        );

        assert!(output.contains("E001"));
        assert!(output.contains("Expresion incompleta"));
        assert!(output.contains("aqui"));
    }

    #[test]
    fn test_format_type_error() {
        let source = "x:i = \"hello\"";
        let span = Span::new(6, 13);

        let output = format_type_error(
            "Tipo incompatible en asignacion",
            &span,
            source,
            "test.aura",
            "Int",
            "String",
        );

        assert!(output.contains("E101"));
        assert!(output.contains("esperado 'Int'"));
        assert!(output.contains("encontrado 'String'"));
    }

    #[test]
    fn test_format_reference_error() {
        let source = create_test_source();
        let span = Span::new(118, 133); // "generate_report"

        let output = format_reference_error(
            "generate_report",
            &span,
            source,
            "main.aura",
            Some(&["generate_reports", "get_report"]),
        );

        assert!(output.contains("E201"));
        assert!(output.contains("'generate_report' no esta definido"));
        assert!(output.contains("Quizas quisiste decir"));
    }

    #[test]
    fn test_format_capability_error() {
        let source = "fetch!(\"https://api.com\")";
        let span = Span::new(0, 6);

        let output = format_capability_error("http", &span, source, "test.aura");

        assert!(output.contains("E501"));
        assert!(output.contains("+http"));
        assert!(output.contains("Agrega '+http'"));
    }

    #[test]
    fn test_format_effect_error() {
        let source = "get_data(url) = http.get(url)";
        let span = Span::new(0, 8);

        let output = format_effect_error("get_data", &span, source, "test.aura");

        assert!(output.contains("E301"));
        assert!(output.contains("efectos"));
        assert!(output.contains("get_data!(...)"));
    }

    #[test]
    fn test_format_error_pretty_with_suggestion() {
        let source = create_test_source();

        let error = AuraError::new(
            ErrorCode::reference(1),
            Severity::Error,
            Location::with_range("main.aura", 10, 10, 25),
            "Funcion no definida: 'generate_report'",
        )
        .with_suggestion(
            "Definir la funcion",
            Some("generate_report(users) = ...".to_string()),
        );

        let output = format_error_pretty(&error, source, "main.aura");

        assert!(output.contains("E201"));
        assert!(output.contains("generate_report"));
        assert!(output.contains("Definir la funcion"));
    }

    #[test]
    fn test_format_errors_pretty_multiple() {
        let source = "x = foo\ny = bar";

        let errors = vec![
            AuraError::new(
                ErrorCode::reference(1),
                Severity::Error,
                Location::with_range("test.aura", 1, 5, 8),
                "'foo' no esta definido",
            ),
            AuraError::new(
                ErrorCode::reference(1),
                Severity::Warning,
                Location::with_range("test.aura", 2, 5, 8),
                "'bar' no esta definido",
            ),
        ];

        let output = format_errors_pretty(&errors, source, "test.aura");

        assert!(output.contains("foo"));
        assert!(output.contains("bar"));
        assert!(output.contains("Total: 1 error(es), 1 advertencia(s)"));
    }

    #[test]
    fn test_error_type_from_code() {
        assert_eq!(ErrorType::from_code("E001"), ErrorType::Parse);
        assert_eq!(ErrorType::from_code("E101"), ErrorType::Type);
        assert_eq!(ErrorType::from_code("E201"), ErrorType::Reference);
        assert_eq!(ErrorType::from_code("E301"), ErrorType::Effect);
        assert_eq!(ErrorType::from_code("E401"), ErrorType::Runtime);
        assert_eq!(ErrorType::from_code("E501"), ErrorType::Capability);
        assert_eq!(ErrorType::from_code("E901"), ErrorType::Agent);
    }

    #[test]
    fn test_location_to_span() {
        let source = "linea uno\nlinea dos\nlinea tres";

        // Linea 2, columna 7 (palabra "dos")
        let location = Location::with_range("test.aura", 2, 7, 10);

        let span = location_to_span(&location, source);

        // "linea uno\n" = 10 chars, + 6 = 16
        assert_eq!(span.start, 16);
        assert_eq!(span.end, 19);
    }

    #[test]
    fn test_format_error_without_end_col() {
        let source = "x = undefined_var";

        let error = AuraError::new(
            ErrorCode::reference(1),
            Severity::Error,
            Location::simple("test.aura", 1, 5),
            "Variable no definida",
        );

        let output = format_error_pretty(&error, source, "test.aura");

        // Debe funcionar sin end_col
        assert!(output.contains("E201"));
        assert!(output.contains("Variable no definida"));
    }
}
