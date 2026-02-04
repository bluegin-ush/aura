use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "aura")]
#[command(about = "AURA - Agent-Unified Runtime Architecture")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Ejecuta un archivo AURA
    Run {
        /// Archivo a ejecutar
        file: PathBuf,
    },

    /// Tokeniza un archivo (debug)
    Lex {
        /// Archivo a tokenizar
        file: PathBuf,

        /// Salida en JSON
        #[arg(long)]
        json: bool,
    },

    /// Parsea un archivo (debug)
    Parse {
        /// Archivo a parsear
        file: PathBuf,

        /// Salida en JSON
        #[arg(long)]
        json: bool,
    },

    /// Verifica tipos sin ejecutar
    Check {
        /// Archivo a verificar
        file: PathBuf,
    },

    /// REPL interactivo
    Repl,

    /// Información del runtime
    Info {
        /// Salida en JSON
        #[arg(long)]
        json: bool,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Run { file } => {
            run_file(&file);
        }
        Commands::Lex { file, json } => {
            lex_file(&file, json);
        }
        Commands::Parse { file, json } => {
            parse_file(&file, json);
        }
        Commands::Check { file } => {
            check_file(&file);
        }
        Commands::Repl => {
            run_repl();
        }
        Commands::Info { json } => {
            show_info(json);
        }
    }
}

fn run_file(path: &PathBuf) {
    let source = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error leyendo archivo: {}", e);
            std::process::exit(1);
        }
    };

    // Tokenizar
    let tokens = match aura::tokenize(&source) {
        Ok(t) => t,
        Err(errors) => {
            eprintln!("Errores de tokenización:");
            for e in errors {
                eprintln!("  [{}-{}]: {}", e.span.start, e.span.end, e.message);
            }
            std::process::exit(1);
        }
    };

    // Parsear
    let program = match aura::parse(tokens) {
        Ok(p) => p,
        Err(errors) => {
            eprintln!("Errores de parsing:");
            for e in errors {
                eprintln!("  {}", e.message);
            }
            std::process::exit(1);
        }
    };

    // Ejecutar
    let mut vm = aura::vm::VM::new();
    vm.load(&program);

    match vm.run() {
        Ok(result) => {
            println!("{}", result);
        }
        Err(e) => {
            eprintln!("Error de ejecución: {}", e.message);
            std::process::exit(1);
        }
    }
}

fn lex_file(path: &PathBuf, json: bool) {
    let source = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error leyendo archivo: {}", e);
            std::process::exit(1);
        }
    };

    if json {
        println!("{}", aura::lexer::tokenize_json(&source));
    } else {
        match aura::tokenize(&source) {
            Ok(tokens) => {
                for t in tokens {
                    println!("[{:4}-{:4}] {:?}", t.span.start, t.span.end, t.value);
                }
            }
            Err(errors) => {
                for e in errors {
                    eprintln!("Error [{}-{}]: {}", e.span.start, e.span.end, e.message);
                }
                std::process::exit(1);
            }
        }
    }
}

fn parse_file(path: &PathBuf, json: bool) {
    let source = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error leyendo archivo: {}", e);
            std::process::exit(1);
        }
    };

    let tokens = match aura::tokenize(&source) {
        Ok(t) => t,
        Err(errors) => {
            eprintln!("Errores de tokenización:");
            for e in errors {
                eprintln!("  {}", e.message);
            }
            std::process::exit(1);
        }
    };

    if json {
        println!("{}", aura::parser::parse_json(tokens));
    } else {
        match aura::parse(tokens) {
            Ok(program) => {
                println!("Programa parseado exitosamente:");
                println!();
                println!("Capacidades: {:?}", program.capabilities.iter().map(|c| &c.name).collect::<Vec<_>>());
                println!();
                for def in &program.definitions {
                    match def {
                        aura::Definition::TypeDef(t) => {
                            println!("@{} {{", t.name);
                            for f in &t.fields {
                                println!("  {}: {:?} {:?}", f.name, f.ty, f.annotations.iter().map(|a| &a.name).collect::<Vec<_>>());
                            }
                            println!("}}");
                        }
                        aura::Definition::FuncDef(f) => {
                            let effect = if f.has_effect { "!" } else { "" };
                            println!(
                                "{}{}({}) = <expr>",
                                f.name,
                                effect,
                                f.params.iter().map(|p| p.name.as_str()).collect::<Vec<_>>().join(" ")
                            );
                        }
                        _ => {}
                    }
                }
            }
            Err(errors) => {
                eprintln!("Errores de parsing:");
                for e in errors {
                    eprintln!("  {}", e.message);
                }
                std::process::exit(1);
            }
        }
    }
}

fn check_file(path: &PathBuf) {
    let source = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error leyendo archivo: {}", e);
            std::process::exit(1);
        }
    };

    // Tokenizar
    let tokens = match aura::tokenize(&source) {
        Ok(t) => t,
        Err(errors) => {
            eprintln!("Errores de tokenización:");
            for e in errors {
                eprintln!("  {}", e.message);
            }
            std::process::exit(1);
        }
    };

    // Parsear
    let program = match aura::parse(tokens) {
        Ok(p) => p,
        Err(errors) => {
            eprintln!("Errores de parsing:");
            for e in errors {
                eprintln!("  {}", e.message);
            }
            std::process::exit(1);
        }
    };

    // Type check
    match aura::types::check(&program) {
        Ok(()) => {
            println!("✓ Programa válido");
            println!("  {} capacidades", program.capabilities.len());
            println!("  {} definiciones", program.definitions.len());
        }
        Err(errors) => {
            eprintln!("Errores de tipos:");
            for e in errors {
                eprintln!("  {}", e.message);
                if let Some(suggestion) = &e.suggestion {
                    eprintln!("    Sugerencia: {}", suggestion);
                }
            }
            std::process::exit(1);
        }
    }
}

fn run_repl() {
    println!("AURA REPL v{}", aura::VERSION);
    println!("Escribe 'exit' para salir, ':reset' para reiniciar, '?help' para ayuda\n");

    let stdin = std::io::stdin();
    let mut line = String::new();

    // Crear VM persistente que mantiene el estado entre lineas
    let mut vm = aura::vm::VM::new();

    loop {
        print!("> ");
        use std::io::Write;
        std::io::stdout().flush().unwrap();

        line.clear();
        if stdin.read_line(&mut line).is_err() {
            break;
        }

        let input = line.trim();

        // Comandos de salida
        if input == "exit" || input == "quit" {
            break;
        }

        if input.is_empty() {
            continue;
        }

        // Comando especial :reset
        if input == ":reset" {
            vm.reset();
            println!("Estado reiniciado");
            continue;
        }

        // Comandos de introspeccion
        if input.starts_with('?') {
            handle_introspection(input, &vm);
            continue;
        }

        // Tokenizar input
        let tokens = match aura::tokenize(input) {
            Ok(t) => t,
            Err(errors) => {
                for e in errors {
                    eprintln!("Error de sintaxis: {}", e.message);
                }
                continue;
            }
        };

        // Determinar si es una definicion de funcion o una expresion
        if aura::looks_like_function_def(&tokens) {
            // Parsear como definicion de funcion
            match aura::parse_function_def(tokens) {
                Ok(func_def) => {
                    let name = func_def.name.clone();
                    vm.define_function(func_def);
                    println!("<fn {}>", name);
                }
                Err(e) => {
                    eprintln!("Error de parsing: {}", e.message);
                }
            }
        } else {
            // Parsear y evaluar como expresion
            match aura::parse_expression(tokens) {
                Ok(expr) => {
                    match vm.eval(&expr) {
                        Ok(value) => {
                            // No mostrar nil para evitar ruido
                            if value != aura::Value::Nil {
                                println!("{}", value);
                            }
                        }
                        Err(e) => {
                            eprintln!("Error de ejecucion: {}", e.message);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error de parsing: {}", e.message);
                }
            }
        }
    }
}

fn handle_introspection(cmd: &str, vm: &aura::vm::VM) {
    match cmd {
        "?types" => println!("Tipos definidos: (ninguno aun)"),
        "?funcs" => {
            let funcs = vm.list_functions();
            if funcs.is_empty() {
                println!("Funciones definidas: (ninguna)");
            } else {
                println!("Funciones definidas: {}", funcs.join(", "));
            }
        }
        "?vars" => {
            let vars = vm.list_variables();
            if vars.is_empty() {
                println!("Variables definidas: (ninguna)");
            } else {
                println!("Variables definidas: {}", vars.join(", "));
            }
        }
        "?caps" => println!("Capacidades: http, json, db, auth, ws, fs, crypto, time, email"),
        "?help" => {
            println!("Comandos de introspeccion:");
            println!("  ?types  - Lista tipos definidos");
            println!("  ?funcs  - Lista funciones definidas");
            println!("  ?vars   - Lista variables definidas");
            println!("  ?caps   - Lista capacidades disponibles");
            println!("  ?help   - Muestra esta ayuda");
            println!();
            println!("Comandos especiales:");
            println!("  :reset  - Reinicia el estado de la sesion");
            println!("  exit    - Sale del REPL");
        }
        _ => println!("Comando desconocido. Usa ?help"),
    }
}

fn show_info(json: bool) {
    if json {
        println!("{}", serde_json::to_string_pretty(&aura::runtime_info()).unwrap());
    } else {
        println!("AURA v{}", aura::VERSION);
        println!();
        println!("Agent-Unified Runtime Architecture");
        println!("Un lenguaje de programación para agentes de IA");
        println!();
        println!("Capacidades disponibles:");
        println!("  +http   - Cliente HTTP");
        println!("  +json   - Serialización JSON");
        println!("  +db     - Base de datos");
        println!("  +auth   - Autenticación");
        println!("  +ws     - WebSockets");
        println!("  +fs     - Sistema de archivos");
        println!("  +crypto - Criptografía");
        println!("  +time   - Fecha/hora");
        println!("  +email  - Envío de emails");
        println!();
        println!("Características:");
        println!("  - Hot reload");
        println!("  - Agent bridge");
        println!("  - Errores JSON");
        println!("  - Parseo incremental");
    }
}
