use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "aura")]
#[command(about = "AURA - Agent-Unified Runtime Architecture")]
#[command(long_about = "AURA - Agent-Unified Runtime Architecture\n\n\
    A programming language designed for AI agents.\n\
    All commands support --json flag for structured output.")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Execute an AURA file
    Run {
        /// File to execute
        file: PathBuf,

        /// Output result as structured JSON (agent-friendly)
        #[arg(long, help = "Output structured JSON with result, type, and duration")]
        json: bool,
    },

    /// Self-healing demo: run file, detect errors, fix automatically
    Heal {
        /// File to heal and execute
        file: PathBuf,

        /// Provider to use (mock, claude, ollama)
        #[arg(short, long, default_value = "mock")]
        provider: String,

        /// Actually apply the fix to the file
        #[arg(long)]
        apply: bool,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Tokenize a file (debug)
    Lex {
        /// File to tokenize
        file: PathBuf,

        /// Output tokens as JSON
        #[arg(long)]
        json: bool,
    },

    /// Parse a file (debug)
    Parse {
        /// File to parse
        file: PathBuf,

        /// Output AST as JSON
        #[arg(long)]
        json: bool,
    },

    /// Type-check a file without executing
    Check {
        /// File to check
        file: PathBuf,

        /// Output result as structured JSON (agent-friendly)
        #[arg(long, help = "Output structured JSON with errors and warnings")]
        json: bool,
    },

    /// Interactive REPL
    Repl,

    /// Runtime information
    Info {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Start HTTP server
    Serve {
        /// AURA file with route definitions
        file: PathBuf,

        /// Port to listen on
        #[arg(short, long, default_value = "8080")]
        port: u16,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Revert the last healing action
    Undo {
        /// List undo history instead of reverting
        #[arg(long)]
        list: bool,

        /// Revert to a specific snapshot ID
        #[arg(long)]
        to: Option<String>,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Manage snapshots
    Snapshots {
        #[command(subcommand)]
        action: Option<SnapshotsAction>,

        /// Output as JSON
        #[arg(long, global = true)]
        json: bool,
    },
}

#[derive(Subcommand)]
enum SnapshotsAction {
    /// Create a manual snapshot
    Create {
        /// Description for the snapshot
        #[arg(short, long)]
        description: Option<String>,

        /// Files to include in snapshot (defaults to current directory .aura files)
        files: Vec<PathBuf>,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Restore a specific snapshot
    Restore {
        /// Snapshot ID to restore
        id: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Remove old snapshots
    Prune {
        /// Number of snapshots to keep (default: 10)
        #[arg(short, long, default_value = "10")]
        keep: usize,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}

fn main() {
    // Load environment variables from .env file if present
    aura::caps::load_dotenv();

    let cli = Cli::parse();

    match cli.command {
        Commands::Run { file, json } => {
            run_file(&file, json);
        }
        Commands::Heal { file, provider, apply, json } => {
            heal_file(&file, &provider, apply, json);
        }
        Commands::Lex { file, json } => {
            lex_file(&file, json);
        }
        Commands::Parse { file, json } => {
            parse_file(&file, json);
        }
        Commands::Check { file, json } => {
            check_file(&file, json);
        }
        Commands::Repl => {
            run_repl();
        }
        Commands::Info { json } => {
            show_info(json);
        }
        Commands::Serve { file, port, json } => {
            serve_file(&file, port, json);
        }
        Commands::Undo { list, to, json } => {
            handle_undo(list, to, json);
        }
        Commands::Snapshots { action, json } => {
            handle_snapshots(action, json);
        }
    }
}

fn run_file(path: &PathBuf, json_output: bool) {
    use aura::cli_output::{JsonError, RunResult, value_to_json};
    use aura::loader;
    use std::time::Instant;

    // Load file with imports resolved
    let program = match loader::load_file(path) {
        Ok(p) => p,
        Err(e) => {
            if json_output {
                let result = RunResult::failure(JsonError::new("E001", &e.message));
                println!("{}", result.to_json());
            } else {
                eprintln!("Error: {}", e);
            }
            std::process::exit(1);
        }
    };

    // Execute with timing
    let mut vm = aura::vm::VM::new();
    vm.load(&program);

    let start = Instant::now();
    match vm.run() {
        Ok(result) => {
            let duration_ms = start.elapsed().as_millis() as u64;
            if json_output {
                let (json_value, type_name) = value_to_json(&result);
                let run_result = RunResult::success(json_value, type_name, duration_ms);
                println!("{}", run_result.to_json());
            } else {
                println!("{}", result);
            }
        }
        Err(e) => {
            if json_output {
                let result = RunResult::failure(JsonError::from_runtime_error(&e));
                println!("{}", result.to_json());
            } else {
                eprintln!("Runtime error: {}", e.message);
            }
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
    use aura::loader;

    // Load file with imports resolved
    let program = match loader::load_file(path) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    };

    if json {
        println!("{}", serde_json::to_string_pretty(&program).unwrap_or_default());
    } else {
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
}

fn check_file(path: &PathBuf, json_output: bool) {
    use aura::cli_output::{CheckResult, JsonError};
    use aura::loader;

    let filename = path.display().to_string();

    // Read source for error reporting
    let source = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            if json_output {
                let result = CheckResult::failure(&filename, vec![
                    JsonError::file_error(format!("Error reading file: {}", e))
                ]);
                println!("{}", result.to_json());
            } else {
                eprintln!("Error reading file: {}", e);
            }
            std::process::exit(1);
        }
    };

    // Load file with imports resolved
    let program = match loader::load_file(path) {
        Ok(p) => p,
        Err(e) => {
            if json_output {
                let result = CheckResult::failure(&filename, vec![
                    JsonError::new("E001", &e.message)
                ]);
                println!("{}", result.to_json());
            } else {
                eprintln!("Error: {}", e);
            }
            std::process::exit(1);
        }
    };

    // Type check
    match aura::types::check(&program) {
        Ok(()) => {
            if json_output {
                let result = CheckResult::success(
                    &filename,
                    program.capabilities.len(),
                    program.definitions.len(),
                );
                println!("{}", result.to_json());
            } else {
                println!("Valid program");
                println!("  {} capabilities", program.capabilities.len());
                println!("  {} definitions", program.definitions.len());
            }
        }
        Err(errors) => {
            if json_output {
                let json_errors: Vec<JsonError> = errors
                    .iter()
                    .map(|e| JsonError::from_type_error(e, &source))
                    .collect();
                let result = CheckResult::failure(&filename, json_errors);
                println!("{}", result.to_json());
            } else {
                eprintln!("Type errors:");
                for e in errors {
                    eprintln!("  {}", e.message);
                    if let Some(suggestion) = &e.suggestion {
                        eprintln!("    Suggestion: {}", suggestion);
                    }
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
        "?caps" => println!("Capacidades: http, json, db, env, auth, ws, fs, crypto, time, email"),
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

/// Storage module for persisting snapshots and undo state
mod storage {
    use std::path::PathBuf;
    use std::fs;
    use serde::{Deserialize, Serialize};

    const AURA_DIR: &str = ".aura";
    const SNAPSHOTS_DIR: &str = "snapshots";
    const UNDO_STATE_FILE: &str = "undo_state.json";

    /// Get the .aura directory path (creates if doesn't exist)
    pub fn get_aura_dir() -> std::io::Result<PathBuf> {
        let path = PathBuf::from(AURA_DIR);
        if !path.exists() {
            fs::create_dir_all(&path)?;
        }
        Ok(path)
    }

    /// Get the snapshots directory path (creates if doesn't exist)
    pub fn get_snapshots_dir() -> std::io::Result<PathBuf> {
        let path = get_aura_dir()?.join(SNAPSHOTS_DIR);
        if !path.exists() {
            fs::create_dir_all(&path)?;
        }
        Ok(path)
    }

    /// Persisted undo state
    #[derive(Debug, Clone, Serialize, Deserialize, Default)]
    pub struct PersistedUndoState {
        pub actions: Vec<PersistedHealingAction>,
        pub current_position: usize,
    }

    /// Persisted healing action
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct PersistedHealingAction {
        pub snapshot_id: String,
        pub timestamp: u64,
        pub file_path: String,
        pub old_code: String,
        pub new_code: String,
        pub confidence: f32,
    }

    /// Persisted snapshot
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct PersistedSnapshot {
        pub id: String,
        pub timestamp: u64,
        pub reason: String,
        pub files: Vec<PersistedFileSnapshot>,
    }

    /// Persisted file snapshot
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct PersistedFileSnapshot {
        pub path: String,
        pub content: String,
        pub hash: String,
    }

    /// Load undo state from disk
    pub fn load_undo_state() -> std::io::Result<PersistedUndoState> {
        let path = get_aura_dir()?.join(UNDO_STATE_FILE);
        if !path.exists() {
            return Ok(PersistedUndoState::default());
        }
        let content = fs::read_to_string(&path)?;
        serde_json::from_str(&content).map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string())
        })
    }

    /// Save undo state to disk
    pub fn save_undo_state(state: &PersistedUndoState) -> std::io::Result<()> {
        let path = get_aura_dir()?.join(UNDO_STATE_FILE);
        let content = serde_json::to_string_pretty(state).map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string())
        })?;
        fs::write(&path, content)
    }

    /// Get snapshot file path
    fn snapshot_path(id: &str) -> std::io::Result<PathBuf> {
        Ok(get_snapshots_dir()?.join(format!("{}.json", id)))
    }

    /// Load a snapshot from disk
    pub fn load_snapshot(id: &str) -> std::io::Result<PersistedSnapshot> {
        let path = snapshot_path(id)?;
        let content = fs::read_to_string(&path)?;
        serde_json::from_str(&content).map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string())
        })
    }

    /// Save a snapshot to disk
    pub fn save_snapshot(snapshot: &PersistedSnapshot) -> std::io::Result<()> {
        let path = snapshot_path(&snapshot.id)?;
        let content = serde_json::to_string_pretty(snapshot).map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string())
        })?;
        fs::write(&path, content)
    }

    /// Delete a snapshot from disk
    pub fn delete_snapshot(id: &str) -> std::io::Result<()> {
        let path = snapshot_path(id)?;
        if path.exists() {
            fs::remove_file(&path)?;
        }
        Ok(())
    }

    /// List all snapshot IDs from disk
    pub fn list_snapshot_ids() -> std::io::Result<Vec<String>> {
        let dir = get_snapshots_dir()?;
        let mut ids = Vec::new();

        for entry in fs::read_dir(&dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().map(|e| e == "json").unwrap_or(false) {
                if let Some(stem) = path.file_stem() {
                    if let Some(s) = stem.to_str() {
                        ids.push(s.to_string());
                    }
                }
            }
        }

        Ok(ids)
    }

    /// List all snapshots from disk
    pub fn list_snapshots() -> std::io::Result<Vec<PersistedSnapshot>> {
        let ids = list_snapshot_ids()?;
        let mut snapshots = Vec::new();

        for id in ids {
            if let Ok(snap) = load_snapshot(&id) {
                snapshots.push(snap);
            }
        }

        // Sort by timestamp (newest first)
        snapshots.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        Ok(snapshots)
    }
}

fn handle_undo(list: bool, to: Option<String>, json_output: bool) {
    use aura::cli_output::{UndoListResult, UndoActionInfo, UndoResult};

    if list {
        // List undo history
        match storage::load_undo_state() {
            Ok(state) => {
                let actions: Vec<UndoActionInfo> = state.actions
                    .iter()
                    .take(state.current_position)
                    .map(|a| UndoActionInfo {
                        id: a.snapshot_id.clone(),
                        timestamp: a.timestamp,
                        file: a.file_path.clone(),
                        patch: a.new_code.clone(),
                        confidence: a.confidence,
                    })
                    .collect();

                if json_output {
                    let result = UndoListResult::success(actions);
                    println!("{}", result.to_json());
                } else {
                    if actions.is_empty() {
                        println!("No actions in undo history");
                    } else {
                        println!("Undo history ({} actions):", actions.len());
                        for action in actions.iter().rev() {
                            println!();
                            println!("  ID: {}", action.id);
                            println!("  File: {}", action.file);
                            println!("  Confidence: {:.0}%", action.confidence * 100.0);
                            println!("  Patch: {}", truncate_str(&action.patch, 50));
                        }
                    }
                }
            }
            Err(e) => {
                if json_output {
                    let result = UndoListResult::failure(e.to_string());
                    println!("{}", result.to_json());
                } else {
                    eprintln!("Error loading undo state: {}", e);
                }
                std::process::exit(1);
            }
        }
        return;
    }

    // Perform undo
    match storage::load_undo_state() {
        Ok(mut state) => {
            if state.current_position == 0 {
                if json_output {
                    let result = UndoResult::failure("Nothing to undo");
                    println!("{}", result.to_json());
                } else {
                    eprintln!("Nothing to undo");
                }
                std::process::exit(1);
            }

            // Determine which action to undo
            let target_idx = if let Some(ref id) = to {
                // Find the action by snapshot ID
                state.actions
                    .iter()
                    .take(state.current_position)
                    .position(|a| a.snapshot_id == *id)
            } else {
                // Undo the last action
                Some(state.current_position - 1)
            };

            match target_idx {
                Some(idx) => {
                    let action = &state.actions[idx];
                    let snapshot_id = action.snapshot_id.clone();

                    // Load the snapshot
                    match storage::load_snapshot(&snapshot_id) {
                        Ok(snapshot) => {
                            let mut restored_files = Vec::new();
                            let mut errors = Vec::new();

                            // Restore each file
                            for file_snap in &snapshot.files {
                                let path = PathBuf::from(&file_snap.path);
                                match std::fs::write(&path, &file_snap.content) {
                                    Ok(_) => restored_files.push(file_snap.path.clone()),
                                    Err(e) => errors.push((file_snap.path.clone(), e.to_string())),
                                }
                            }

                            // Update state - revert to position before this action
                            state.current_position = idx;
                            if let Err(e) = storage::save_undo_state(&state) {
                                if json_output {
                                    let result = UndoResult::failure(format!("Failed to save state: {}", e));
                                    println!("{}", result.to_json());
                                } else {
                                    eprintln!("Warning: Failed to save undo state: {}", e);
                                }
                            }

                            if json_output {
                                let result = UndoResult::success(snapshot_id, restored_files);
                                println!("{}", result.to_json());
                            } else {
                                if errors.is_empty() {
                                    println!("Successfully reverted to snapshot: {}", snapshot.id);
                                    for file in &restored_files {
                                        println!("  Restored: {}", file);
                                    }
                                } else {
                                    println!("Partially reverted to snapshot: {}", snapshot.id);
                                    for file in &restored_files {
                                        println!("  Restored: {}", file);
                                    }
                                    for (file, err) in &errors {
                                        eprintln!("  Failed: {} ({})", file, err);
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            if json_output {
                                let result = UndoResult::failure(format!("Snapshot not found: {}", e));
                                println!("{}", result.to_json());
                            } else {
                                eprintln!("Error loading snapshot: {}", e);
                            }
                            std::process::exit(1);
                        }
                    }
                }
                None => {
                    if json_output {
                        let result = UndoResult::failure(format!("Snapshot not found: {}", to.unwrap_or_default()));
                        println!("{}", result.to_json());
                    } else {
                        eprintln!("Snapshot not found: {}", to.unwrap_or_default());
                    }
                    std::process::exit(1);
                }
            }
        }
        Err(e) => {
            if json_output {
                let result = UndoResult::failure(e.to_string());
                println!("{}", result.to_json());
            } else {
                eprintln!("Error loading undo state: {}", e);
            }
            std::process::exit(1);
        }
    }
}

fn handle_snapshots(action: Option<SnapshotsAction>, parent_json: bool) {
    use aura::cli_output::{
        SnapshotsListResult, SnapshotInfo, SnapshotCreateResult,
        SnapshotRestoreResult, SnapshotRestoreFailure, SnapshotPruneResult,
    };
    use std::time::{SystemTime, UNIX_EPOCH};

    match action {
        None => {
            // List all snapshots (use parent_json for the list command)
            let json_output = parent_json;

            match storage::list_snapshots() {
                Ok(snapshots) => {
                    let infos: Vec<SnapshotInfo> = snapshots
                        .iter()
                        .map(|s| SnapshotInfo {
                            id: s.id.clone(),
                            timestamp: s.timestamp,
                            reason: s.reason.clone(),
                            files: s.files.iter().map(|f| f.path.clone()).collect(),
                        })
                        .collect();

                    if json_output {
                        let result = SnapshotsListResult::success(infos);
                        println!("{}", result.to_json());
                    } else {
                        if infos.is_empty() {
                            println!("No snapshots found");
                        } else {
                            println!("Snapshots ({}):", infos.len());
                            for info in &infos {
                                println!();
                                println!("  ID: {}", info.id);
                                println!("  Reason: {}", info.reason);
                                println!("  Files: {}", info.files.join(", "));
                            }
                        }
                    }
                }
                Err(e) => {
                    if json_output {
                        let result = SnapshotsListResult::failure(e.to_string());
                        println!("{}", result.to_json());
                    } else {
                        eprintln!("Error listing snapshots: {}", e);
                    }
                    std::process::exit(1);
                }
            }
        }

        Some(SnapshotsAction::Create { description, files, json }) => {
            let json_output = json || parent_json;
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();

            let id = format!("snap_{}", SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos());

            let reason = description.unwrap_or_else(|| "Manual snapshot".to_string());

            // Determine files to snapshot
            let files_to_snap: Vec<PathBuf> = if files.is_empty() {
                // Default: find all .aura files in current directory
                match std::fs::read_dir(".") {
                    Ok(entries) => entries
                        .filter_map(|e| e.ok())
                        .map(|e| e.path())
                        .filter(|p| p.extension().map(|e| e == "aura").unwrap_or(false))
                        .collect(),
                    Err(_) => Vec::new(),
                }
            } else {
                files
            };

            if files_to_snap.is_empty() {
                if json_output {
                    let result = SnapshotCreateResult::failure("No files to snapshot");
                    println!("{}", result.to_json());
                } else {
                    eprintln!("No files to snapshot");
                }
                std::process::exit(1);
            }

            // Read file contents
            let mut file_snapshots = Vec::new();
            let mut file_names = Vec::new();
            for path in &files_to_snap {
                match std::fs::read_to_string(path) {
                    Ok(content) => {
                        let path_str = path.display().to_string();
                        file_names.push(path_str.clone());

                        // Compute simple hash
                        let mut hash: u64 = 0;
                        for byte in content.bytes() {
                            hash = hash.wrapping_mul(31).wrapping_add(byte as u64);
                        }

                        file_snapshots.push(storage::PersistedFileSnapshot {
                            path: path_str,
                            content,
                            hash: format!("{:016x}", hash),
                        });
                    }
                    Err(e) => {
                        if json_output {
                            let result = SnapshotCreateResult::failure(
                                format!("Failed to read {}: {}", path.display(), e)
                            );
                            println!("{}", result.to_json());
                        } else {
                            eprintln!("Failed to read {}: {}", path.display(), e);
                        }
                        std::process::exit(1);
                    }
                }
            }

            let snapshot = storage::PersistedSnapshot {
                id: id.clone(),
                timestamp,
                reason: reason.clone(),
                files: file_snapshots,
            };

            match storage::save_snapshot(&snapshot) {
                Ok(_) => {
                    if json_output {
                        let result = SnapshotCreateResult::success(&id, timestamp, file_names);
                        println!("{}", result.to_json());
                    } else {
                        println!("Created snapshot: {}", id);
                        println!("  Reason: {}", reason);
                        println!("  Files: {}", file_names.join(", "));
                    }
                }
                Err(e) => {
                    if json_output {
                        let result = SnapshotCreateResult::failure(e.to_string());
                        println!("{}", result.to_json());
                    } else {
                        eprintln!("Failed to save snapshot: {}", e);
                    }
                    std::process::exit(1);
                }
            }
        }

        Some(SnapshotsAction::Restore { id, json }) => {
            let json_output = json || parent_json;

            match storage::load_snapshot(&id) {
                Ok(snapshot) => {
                    let mut restored = Vec::new();
                    let mut failed = Vec::new();

                    for file_snap in &snapshot.files {
                        let path = PathBuf::from(&file_snap.path);
                        match std::fs::write(&path, &file_snap.content) {
                            Ok(_) => restored.push(file_snap.path.clone()),
                            Err(e) => failed.push(SnapshotRestoreFailure {
                                file: file_snap.path.clone(),
                                reason: e.to_string(),
                            }),
                        }
                    }

                    if json_output {
                        let result = SnapshotRestoreResult::success(&id, restored.clone(), failed.clone());
                        println!("{}", result.to_json());
                    } else {
                        if failed.is_empty() {
                            println!("Restored snapshot: {}", id);
                            for file in &restored {
                                println!("  Restored: {}", file);
                            }
                        } else {
                            println!("Partially restored snapshot: {}", id);
                            for file in &restored {
                                println!("  Restored: {}", file);
                            }
                            for fail in &failed {
                                eprintln!("  Failed: {} ({})", fail.file, fail.reason);
                            }
                        }
                    }

                    if !failed.is_empty() {
                        std::process::exit(1);
                    }
                }
                Err(e) => {
                    if json_output {
                        let result = SnapshotRestoreResult::failure(format!("Snapshot not found: {}", e));
                        println!("{}", result.to_json());
                    } else {
                        eprintln!("Error loading snapshot: {}", e);
                    }
                    std::process::exit(1);
                }
            }
        }

        Some(SnapshotsAction::Prune { keep, json }) => {
            let json_output = json || parent_json;

            match storage::list_snapshots() {
                Ok(snapshots) => {
                    let total = snapshots.len();

                    // Remove oldest snapshots (they are sorted newest first)
                    let mut removed = 0;
                    for snapshot in snapshots.iter().skip(keep) {
                        if let Err(e) = storage::delete_snapshot(&snapshot.id) {
                            if !json_output {
                                eprintln!("Warning: Failed to delete {}: {}", snapshot.id, e);
                            }
                        } else {
                            removed += 1;
                        }
                    }

                    let remaining = total - removed;

                    if json_output {
                        let result = SnapshotPruneResult::success(removed, remaining);
                        println!("{}", result.to_json());
                    } else {
                        println!("Pruned {} snapshots, {} remaining", removed, remaining);
                    }
                }
                Err(e) => {
                    if json_output {
                        let result = SnapshotPruneResult::failure(e.to_string());
                        println!("{}", result.to_json());
                    } else {
                        eprintln!("Error listing snapshots: {}", e);
                    }
                    std::process::exit(1);
                }
            }
        }
    }
}

/// Truncate a string for display
fn truncate_str(s: &str, max_len: usize) -> String {
    let s = s.replace('\n', " ").replace('\r', "");
    if s.len() > max_len {
        format!("{}...", &s[..max_len])
    } else {
        s
    }
}

/// Serve an AURA file as HTTP server
fn serve_file(path: &PathBuf, port: u16, json_output: bool) {
    use aura::server::start_server;
    use aura::loader;

    // Load file with imports resolved
    let program = match loader::load_file(path) {
        Ok(p) => p,
        Err(e) => {
            if json_output {
                println!(r#"{{"success":false,"error":"{}"}}"#, e.message.replace('"', "\\\""));
            } else {
                eprintln!("Error: {}", e);
            }
            std::process::exit(1);
        }
    };

    // Extract routes from function definitions
    // Convention: get_users -> GET /users, post_user -> POST /user, etc.
    let routes = extract_routes(&program);

    if routes.is_empty() {
        if json_output {
            println!(r#"{{"success":false,"error":"No routes found. Define functions like get_users, post_user, etc."}}"#);
        } else {
            eprintln!("No routes found.");
            eprintln!("Define functions following REST convention:");
            eprintln!("  get_users     -> GET /users");
            eprintln!("  get_user(id)  -> GET /user/:id");
            eprintln!("  post_user     -> POST /user");
            eprintln!("  put_user(id)  -> PUT /user/:id");
            eprintln!("  del_user(id)  -> DELETE /user/:id");
        }
        std::process::exit(1);
    }

    if !json_output {
        println!("Starting AURA server on port {}...", port);
        println!("Routes:");
    }

    // Run async server
    let rt = tokio::runtime::Runtime::new().expect("Failed to create runtime");
    rt.block_on(async {
        if let Err(e) = start_server(port, routes, program).await {
            if json_output {
                println!(r#"{{"success":false,"error":"Server error: {}"}}"#, e);
            } else {
                eprintln!("Server error: {}", e);
            }
            std::process::exit(1);
        }
    });
}

/// Extract routes from function definitions based on naming convention
fn extract_routes(program: &aura::Program) -> Vec<aura::server::Route> {
    let mut routes = Vec::new();

    for def in &program.definitions {
        if let aura::Definition::FuncDef(func) = def {
            if let Some(route) = parse_route_from_name(&func.name, &func.params) {
                routes.push(route);
            }
        }
    }

    routes
}

/// Parse route from function name following REST convention
/// get_users -> GET /users
/// get_user(id) -> GET /user/:id
/// post_user -> POST /user
fn parse_route_from_name(name: &str, params: &[aura::parser::Param]) -> Option<aura::server::Route> {
    let prefixes = [
        ("get_", "GET"),
        ("post_", "POST"),
        ("put_", "PUT"),
        ("patch_", "PATCH"),
        ("del_", "DELETE"),
        ("delete_", "DELETE"),
    ];

    for (prefix, method) in prefixes {
        if name.starts_with(prefix) {
            let resource = &name[prefix.len()..];
            let path = build_path(resource, params);
            return Some(aura::server::Route::new(method, &path, name));
        }
    }

    None
}

/// Build path from resource name and parameters
/// "users" + [] -> "/users"
/// "user" + [id] -> "/user/:id"
fn build_path(resource: &str, params: &[aura::parser::Param]) -> String {
    let mut path = format!("/{}", resource.replace('_', "/"));

    for param in params {
        // Skip 'req' parameter (request object)
        if param.name != "req" {
            path.push_str(&format!("/:{}", param.name));
        }
    }

    path
}

/// Self-healing demo command
fn heal_file(path: &PathBuf, provider: &str, apply: bool, json_output: bool) {
    use std::io::Write;
    use std::thread;
    use std::time::Duration;

    // ANSI colors
    const RED: &str = "\x1b[31m";
    const GREEN: &str = "\x1b[32m";
    const YELLOW: &str = "\x1b[33m";
    const BLUE: &str = "\x1b[34m";
    const CYAN: &str = "\x1b[36m";
    const BOLD: &str = "\x1b[1m";
    const DIM: &str = "\x1b[2m";
    const RESET: &str = "\x1b[0m";

    fn print_step(icon: &str, color: &str, msg: &str) {
        println!("{}{} {}{}", color, icon, msg, RESET);
    }

    fn print_code(code: &str, highlight_error: bool) {
        for (i, line) in code.lines().enumerate() {
            let line_num = i + 1;
            if highlight_error && line.contains("ERROR_HERE") {
                println!("  {}{}  {}  {}{}", RED, line_num, line, RESET, "");
            } else {
                println!("  {}{}{}  {}", DIM, line_num, RESET, line);
            }
        }
    }

    fn spinner(msg: &str, duration_ms: u64) {
        let frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
        let iterations = (duration_ms / 80) as usize;
        for i in 0..iterations {
            print!("\r{}{}  {}{}", CYAN, frames[i % frames.len()], msg, RESET);
            std::io::stdout().flush().unwrap();
            thread::sleep(Duration::from_millis(80));
        }
        print!("\r");
        std::io::stdout().flush().unwrap();
    }

    // Read file
    let source = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            if json_output {
                println!(r#"{{"success":false,"stage":"read","error":"{}"}}"#, e);
            } else {
                print_step("✗", RED, &format!("Error reading file: {}", e));
            }
            std::process::exit(1);
        }
    };

    if !json_output {
        println!();
        println!("{}{}═══════════════════════════════════════════════════════════════{}", BOLD, BLUE, RESET);
        println!("{}{}   AURA Self-Healing Demo{}", BOLD, BLUE, RESET);
        println!("{}{}═══════════════════════════════════════════════════════════════{}", BOLD, BLUE, RESET);
        println!();

        print_step("📄", BLUE, &format!("File: {}", path.display()));
        print_step("🔧", BLUE, &format!("Provider: {}", provider));
        println!();
    }

    // Step 1: Show original code
    if !json_output {
        print_step("1️⃣", YELLOW, "Original code:");
        println!();
        print_code(&source, false);
        println!();
        thread::sleep(Duration::from_millis(500));
    }

    // Step 2: Try to execute
    if !json_output {
        print_step("2️⃣", YELLOW, "Attempting to execute...");
        spinner("Parsing and executing", 800);
    }

    // Tokenize
    let tokens = match aura::tokenize(&source) {
        Ok(t) => t,
        Err(errors) => {
            let error_msg = errors.first().map(|e| e.message.clone()).unwrap_or_default();
            if json_output {
                println!(r#"{{"success":false,"stage":"tokenize","error":"{}"}}"#, error_msg);
            } else {
                print_step("❌", RED, &format!("Tokenization error: {}", error_msg));
            }
            std::process::exit(1);
        }
    };

    // Parse
    let program = match aura::parse(tokens) {
        Ok(p) => p,
        Err(errors) => {
            let error_msg = errors.first().map(|e| e.message.clone()).unwrap_or_default();
            if json_output {
                println!(r#"{{"success":false,"stage":"parse","error":"{}"}}"#, error_msg);
            } else {
                print_step("❌", RED, &format!("Parse error: {}", error_msg));
            }
            std::process::exit(1);
        }
    };

    // Execute
    let mut vm = aura::vm::VM::new();
    vm.load(&program);

    match vm.run() {
        Ok(result) => {
            // No error - program runs fine!
            if json_output {
                println!(r#"{{"success":true,"needed_healing":false,"result":"{}"}}"#, result);
            } else {
                println!();
                print_step("✅", GREEN, "Program executed successfully - no healing needed!");
                println!();
                println!("  {}Result:{} {}", BOLD, RESET, result);
                println!();
            }
            return;
        }
        Err(runtime_error) => {
            // Found an error - time to heal!
            if !json_output {
                println!();
                print_step("❌", RED, &format!("Runtime error detected: {}", runtime_error.message));
                println!();
                thread::sleep(Duration::from_millis(500));
            }

            // Step 3: Initiate healing
            if !json_output {
                print_step("3️⃣", YELLOW, "Initiating self-healing...");
                spinner("Analyzing error context", 600);
                println!();
            }

            // Create healing context
            let context = aura::agent::HealingContext::new(
                &source,
                path.display().to_string(),
                1, 1,
            );

            // Step 4: Call the agent
            if !json_output {
                print_step("4️⃣", YELLOW, &format!("Consulting {} agent...", provider));
                spinner("Waiting for AI response", 1200);
            }

            // Use mock provider for demo (real providers require API keys)
            let mock_provider = aura::agent::MockProvider::new()
                .with_latency(0);

            let rt = tokio::runtime::Runtime::new().expect("Failed to create runtime");
            let healing_result = rt.block_on(async {
                let mut engine = aura::agent::HealingEngine::new(mock_provider)
                    .with_auto_apply(true)
                    .with_confidence_threshold(0.5);

                engine.heal_error(&runtime_error, &context).await
            });

            match healing_result {
                Ok(aura::agent::HealingResult::Fixed { patch, explanation }) => {
                    if !json_output {
                        println!();
                        print_step("🔍", CYAN, "Agent analysis:");
                        println!("  {}{}{}", DIM, explanation, RESET);
                        println!();

                        print_step("5️⃣", YELLOW, "Proposed fix:");
                        println!();
                        println!("  {}--- Original{}", RED, RESET);
                        println!("  {}+++ Fixed{}", GREEN, RESET);
                        println!();

                        // Show diff-style output
                        for line in source.lines() {
                            println!("  {}-{} {}{}", RED, RESET, line, RESET);
                        }
                        println!();
                        for line in patch.lines() {
                            println!("  {}+{} {}{}", GREEN, RESET, line, RESET);
                        }
                        println!();

                        thread::sleep(Duration::from_millis(500));
                    }

                    // Step 6: Apply and verify
                    if apply {
                        if !json_output {
                            print_step("6️⃣", YELLOW, "Applying fix to file...");
                        }

                        // Write the fix
                        if let Err(e) = std::fs::write(path, &patch) {
                            if json_output {
                                println!(r#"{{"success":false,"stage":"apply","error":"{}"}}"#, e);
                            } else {
                                print_step("❌", RED, &format!("Failed to write fix: {}", e));
                            }
                            std::process::exit(1);
                        }

                        if !json_output {
                            print_step("✅", GREEN, "Fix applied to file!");
                            println!();
                        }

                        // Try to run again
                        if !json_output {
                            print_step("7️⃣", YELLOW, "Re-executing fixed code...");
                            spinner("Verifying fix", 600);
                        }

                        // Re-run with fixed code
                        let tokens2 = aura::tokenize(&patch).expect("Fixed code should tokenize");
                        let program2 = aura::parse(tokens2).expect("Fixed code should parse");
                        let mut vm2 = aura::vm::VM::new();
                        vm2.load(&program2);

                        match vm2.run() {
                            Ok(result) => {
                                if json_output {
                                    println!(r#"{{"success":true,"needed_healing":true,"fixed":true,"result":"{}","patch":"{}"}}"#,
                                        result,
                                        patch.replace('\n', "\\n").replace('"', "\\\"")
                                    );
                                } else {
                                    println!();
                                    print_step("🎉", GREEN, "SUCCESS! Fixed code executes correctly!");
                                    println!();
                                    println!("  {}Result:{} {}", BOLD, RESET, result);
                                    println!();
                                    println!("{}{}═══════════════════════════════════════════════════════════════{}", BOLD, GREEN, RESET);
                                    println!("{}{}   Self-Healing Complete!{}", BOLD, GREEN, RESET);
                                    println!("{}{}═══════════════════════════════════════════════════════════════{}", BOLD, GREEN, RESET);
                                    println!();
                                }
                            }
                            Err(e) => {
                                if json_output {
                                    println!(r#"{{"success":false,"stage":"verify","error":"{}"}}"#, e.message);
                                } else {
                                    print_step("❌", RED, &format!("Fix didn't work: {}", e.message));
                                }
                                std::process::exit(1);
                            }
                        }
                    } else {
                        // Don't apply, just show the fix
                        if json_output {
                            println!(r#"{{"success":true,"needed_healing":true,"fixed":false,"patch":"{}","explanation":"{}"}}"#,
                                patch.replace('\n', "\\n").replace('"', "\\\""),
                                explanation.replace('"', "\\\"")
                            );
                        } else {
                            println!("  {}Use --apply to write the fix to the file{}", DIM, RESET);
                            println!();
                            println!("{}{}═══════════════════════════════════════════════════════════════{}", BOLD, YELLOW, RESET);
                            println!("{}{}   Fix Ready - Use --apply to apply{}", BOLD, YELLOW, RESET);
                            println!("{}{}═══════════════════════════════════════════════════════════════{}", BOLD, YELLOW, RESET);
                            println!();
                        }
                    }
                }
                Ok(aura::agent::HealingResult::Suggested { suggestions }) => {
                    if json_output {
                        println!(r#"{{"success":true,"needed_healing":true,"fixed":false,"suggestions":{:?}}}"#, suggestions);
                    } else {
                        print_step("💡", YELLOW, "Agent has suggestions:");
                        for s in &suggestions {
                            println!("  - {}", s);
                        }
                    }
                }
                Ok(aura::agent::HealingResult::NeedsHuman { reason }) => {
                    if json_output {
                        println!(r#"{{"success":false,"stage":"heal","needs_human":true,"reason":"{}"}}"#, reason);
                    } else {
                        print_step("👤", YELLOW, &format!("Needs human intervention: {}", reason));
                    }
                }
                Ok(aura::agent::HealingResult::CannotFix { reason }) => {
                    if json_output {
                        println!(r#"{{"success":false,"stage":"heal","cannot_fix":true,"reason":"{}"}}"#, reason);
                    } else {
                        print_step("❌", RED, &format!("Cannot fix: {}", reason));
                    }
                }
                Err(e) => {
                    if json_output {
                        println!(r#"{{"success":false,"stage":"heal","error":"{}"}}"#, e);
                    } else {
                        print_step("❌", RED, &format!("Healing failed: {}", e));
                    }
                    std::process::exit(1);
                }
            }
        }
    }
}
