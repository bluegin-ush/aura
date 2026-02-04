// src/agent/undo.rs
// Sistema de undo para healing seguro

use super::response::Patch;
use super::snapshot::{SnapshotId, SnapshotManager, SnapshotReason, SnapshotError};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// Resultado de verificación de un fix
#[derive(Debug, Clone)]
pub enum VerificationResult {
    /// Fix verificado exitosamente
    Success {
        /// Tests que pasaron
        tests_passed: usize,
    },
    /// Fix falló la verificación
    Failure {
        /// Descripción del error
        error: String,
        /// Tests que fallaron
        tests_failed: usize,
    },
    /// Verificación tardó demasiado
    Timeout,
    /// No se pudo verificar (sin tests)
    Skipped,
}

impl VerificationResult {
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Success { .. } | Self::Skipped)
    }
}

/// Una acción de healing registrada
#[derive(Debug, Clone)]
pub struct HealingAction {
    /// Snapshot creado antes de aplicar el fix
    pub snapshot_before: SnapshotId,
    /// Patch que se aplicó
    pub patch_applied: Patch,
    /// Cuándo se aplicó
    pub timestamp: u64,
    /// Confianza del agente en el fix
    pub confidence: f32,
    /// Resultado de verificación (si se ejecutó)
    pub verified: Option<VerificationResult>,
    /// Archivo afectado
    pub file_path: PathBuf,
}

impl HealingAction {
    pub fn new(snapshot_before: SnapshotId, patch: Patch, confidence: f32, file_path: PathBuf) -> Self {
        Self {
            snapshot_before,
            patch_applied: patch,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            confidence,
            verified: None,
            file_path,
        }
    }

    pub fn with_verification(mut self, result: VerificationResult) -> Self {
        self.verified = Some(result);
        self
    }
}

/// Resultado de operación undo
#[derive(Debug)]
pub struct UndoResult {
    /// Snapshot que se restauró
    pub restored_snapshot: SnapshotId,
    /// Archivos que se restauraron
    pub files_restored: Vec<PathBuf>,
    /// Acción que se deshizo
    pub action_undone: HealingAction,
}

/// Resultado de operación redo
#[derive(Debug)]
pub struct RedoResult {
    /// Acción que se re-aplicó
    pub action_reapplied: HealingAction,
}

/// Errores del sistema de undo
#[derive(Debug)]
pub enum UndoError {
    /// No hay acciones para deshacer
    NothingToUndo,
    /// No hay acciones para rehacer
    NothingToRedo,
    /// Error al restaurar snapshot
    SnapshotError(SnapshotError),
    /// Error de IO al restaurar archivos
    IoError(String),
}

impl std::fmt::Display for UndoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NothingToUndo => write!(f, "Nothing to undo"),
            Self::NothingToRedo => write!(f, "Nothing to redo"),
            Self::SnapshotError(e) => write!(f, "Snapshot error: {}", e),
            Self::IoError(msg) => write!(f, "IO error: {}", msg),
        }
    }
}

impl std::error::Error for UndoError {}

impl From<SnapshotError> for UndoError {
    fn from(e: SnapshotError) -> Self {
        Self::SnapshotError(e)
    }
}

/// Gestor de historial de healing con undo/redo
pub struct UndoManager {
    /// Gestor de snapshots subyacente
    snapshot_manager: SnapshotManager,
    /// Historial de acciones
    history: Vec<HealingAction>,
    /// Posición actual en el historial (para redo)
    current_position: usize,
    /// Máximo de acciones en historial
    max_history: usize,
}

impl UndoManager {
    pub fn new(snapshot_manager: SnapshotManager) -> Self {
        Self {
            snapshot_manager,
            history: Vec::new(),
            current_position: 0,
            max_history: 100,
        }
    }

    pub fn with_max_history(mut self, max: usize) -> Self {
        self.max_history = max;
        self
    }

    /// Crea un snapshot antes de una operación
    pub fn create_snapshot(&mut self, reason: SnapshotReason) -> Result<SnapshotId, SnapshotError> {
        self.snapshot_manager.create_snapshot(reason)
    }

    /// Crea un snapshot con archivos
    pub fn create_snapshot_with_files(
        &mut self,
        reason: SnapshotReason,
        files: Vec<(PathBuf, String)>,
    ) -> Result<SnapshotId, SnapshotError> {
        self.snapshot_manager.create_snapshot_with_files(reason, files)
    }

    /// Registra una acción de healing
    pub fn record_action(&mut self, action: HealingAction) {
        // Si estamos en medio del historial (después de undo), truncar
        if self.current_position < self.history.len() {
            self.history.truncate(self.current_position);
        }

        self.history.push(action);
        self.current_position = self.history.len();

        // Limitar tamaño del historial
        while self.history.len() > self.max_history {
            self.history.remove(0);
            self.current_position = self.current_position.saturating_sub(1);
        }
    }

    /// Verifica si se puede hacer undo
    pub fn can_undo(&self) -> bool {
        self.current_position > 0
    }

    /// Verifica si se puede hacer redo
    pub fn can_redo(&self) -> bool {
        self.current_position < self.history.len()
    }

    /// Obtiene los datos para hacer undo (el snapshot a restaurar)
    /// No restaura automáticamente - el caller debe aplicar los cambios
    pub fn prepare_undo(&mut self) -> Result<(&HealingAction, &super::snapshot::Snapshot), UndoError> {
        if !self.can_undo() {
            return Err(UndoError::NothingToUndo);
        }

        let action = &self.history[self.current_position - 1];
        let snapshot = self.snapshot_manager
            .get_restore_data(&action.snapshot_before)?;

        Ok((action, snapshot))
    }

    /// Confirma que el undo fue aplicado
    pub fn confirm_undo(&mut self) -> Option<HealingAction> {
        if self.current_position > 0 {
            self.current_position -= 1;
            Some(self.history[self.current_position].clone())
        } else {
            None
        }
    }

    /// Obtiene los datos para hacer redo
    pub fn prepare_redo(&self) -> Result<&HealingAction, UndoError> {
        if !self.can_redo() {
            return Err(UndoError::NothingToRedo);
        }

        Ok(&self.history[self.current_position])
    }

    /// Confirma que el redo fue aplicado
    pub fn confirm_redo(&mut self) -> Option<&HealingAction> {
        if self.current_position < self.history.len() {
            let action = &self.history[self.current_position];
            self.current_position += 1;
            Some(action)
        } else {
            None
        }
    }

    /// Obtiene el historial completo
    pub fn get_history(&self) -> &[HealingAction] {
        &self.history
    }

    /// Obtiene las acciones que se pueden deshacer
    pub fn get_undoable_actions(&self) -> &[HealingAction] {
        if self.current_position > 0 {
            &self.history[..self.current_position]
        } else {
            &[]
        }
    }

    /// Obtiene las acciones que se pueden rehacer
    pub fn get_redoable_actions(&self) -> &[HealingAction] {
        if self.current_position < self.history.len() {
            &self.history[self.current_position..]
        } else {
            &[]
        }
    }

    /// Número de acciones en historial
    pub fn history_count(&self) -> usize {
        self.history.len()
    }

    /// Posición actual en el historial
    pub fn current_position(&self) -> usize {
        self.current_position
    }

    /// Acceso al snapshot manager
    pub fn snapshot_manager(&self) -> &SnapshotManager {
        &self.snapshot_manager
    }

    /// Acceso mutable al snapshot manager
    pub fn snapshot_manager_mut(&mut self) -> &mut SnapshotManager {
        &mut self.snapshot_manager
    }

    /// Limpia el historial
    pub fn clear_history(&mut self) {
        self.history.clear();
        self.current_position = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::response::Patch;

    fn create_test_patch() -> Patch {
        Patch::new("old", "new")
            .with_location("test.aura", 1, 1)
    }

    fn create_test_action(snapshot_id: SnapshotId) -> HealingAction {
        HealingAction::new(
            snapshot_id,
            create_test_patch(),
            0.9,
            PathBuf::from("test.aura"),
        )
    }

    #[test]
    fn test_undo_manager_new() {
        let snap_manager = SnapshotManager::new(10);
        let undo_manager = UndoManager::new(snap_manager);

        assert!(!undo_manager.can_undo());
        assert!(!undo_manager.can_redo());
        assert_eq!(undo_manager.history_count(), 0);
    }

    #[test]
    fn test_record_action() {
        let snap_manager = SnapshotManager::new(10);
        let mut undo_manager = UndoManager::new(snap_manager);

        let snap_id = undo_manager.create_snapshot(SnapshotReason::Checkpoint).unwrap();
        let action = create_test_action(snap_id);

        undo_manager.record_action(action);

        assert!(undo_manager.can_undo());
        assert!(!undo_manager.can_redo());
        assert_eq!(undo_manager.history_count(), 1);
    }

    #[test]
    fn test_undo_redo_flow() {
        let snap_manager = SnapshotManager::new(10);
        let mut undo_manager = UndoManager::new(snap_manager);

        // Crear snapshot y acción
        let files = vec![(PathBuf::from("test.aura"), "original content".to_string())];
        let snap_id = undo_manager
            .create_snapshot_with_files(SnapshotReason::BeforeHeal {
                error_id: "err1".to_string(),
            }, files)
            .unwrap();

        let action = create_test_action(snap_id);
        undo_manager.record_action(action);

        // Estado inicial: puede undo, no puede redo
        assert!(undo_manager.can_undo());
        assert!(!undo_manager.can_redo());
        assert_eq!(undo_manager.current_position(), 1);

        // Preparar undo
        let (action, snapshot) = undo_manager.prepare_undo().unwrap();
        assert_eq!(action.file_path, PathBuf::from("test.aura"));
        assert_eq!(
            snapshot.get_file(&PathBuf::from("test.aura")).unwrap().content,
            "original content"
        );

        // Confirmar undo
        undo_manager.confirm_undo();
        assert!(!undo_manager.can_undo());
        assert!(undo_manager.can_redo());
        assert_eq!(undo_manager.current_position(), 0);

        // Confirmar redo
        undo_manager.confirm_redo();
        assert!(undo_manager.can_undo());
        assert!(!undo_manager.can_redo());
        assert_eq!(undo_manager.current_position(), 1);
    }

    #[test]
    fn test_history_truncation_on_new_action() {
        let snap_manager = SnapshotManager::new(10);
        let mut undo_manager = UndoManager::new(snap_manager);

        // Agregar 3 acciones
        for i in 0..3 {
            let snap_id = undo_manager
                .create_snapshot(SnapshotReason::BeforeHeal {
                    error_id: format!("err{}", i),
                })
                .unwrap();
            undo_manager.record_action(create_test_action(snap_id));
        }

        assert_eq!(undo_manager.history_count(), 3);

        // Undo 2 veces
        undo_manager.confirm_undo();
        undo_manager.confirm_undo();
        assert_eq!(undo_manager.current_position(), 1);

        // Agregar nueva acción - debe truncar historial
        let snap_id = undo_manager
            .create_snapshot(SnapshotReason::BeforeHeal {
                error_id: "new_err".to_string(),
            })
            .unwrap();
        undo_manager.record_action(create_test_action(snap_id));

        // Historial debe ser: acción 0 + nueva acción
        assert_eq!(undo_manager.history_count(), 2);
        assert!(!undo_manager.can_redo());
    }

    #[test]
    fn test_max_history_limit() {
        let snap_manager = SnapshotManager::new(100);
        let mut undo_manager = UndoManager::new(snap_manager).with_max_history(5);

        // Agregar 7 acciones
        for i in 0..7 {
            let snap_id = undo_manager
                .create_snapshot(SnapshotReason::BeforeHeal {
                    error_id: format!("err{}", i),
                })
                .unwrap();
            undo_manager.record_action(create_test_action(snap_id));
        }

        // Solo debe haber 5
        assert_eq!(undo_manager.history_count(), 5);
    }

    #[test]
    fn test_verification_result() {
        let success = VerificationResult::Success { tests_passed: 10 };
        let failure = VerificationResult::Failure {
            error: "test failed".to_string(),
            tests_failed: 2,
        };
        let timeout = VerificationResult::Timeout;
        let skipped = VerificationResult::Skipped;

        assert!(success.is_success());
        assert!(!failure.is_success());
        assert!(!timeout.is_success());
        assert!(skipped.is_success());
    }

    #[test]
    fn test_healing_action_with_verification() {
        let snap_id = SnapshotId::new();
        let action = HealingAction::new(
            snap_id,
            create_test_patch(),
            0.85,
            PathBuf::from("test.aura"),
        )
        .with_verification(VerificationResult::Success { tests_passed: 5 });

        assert!(action.verified.is_some());
        assert!(action.verified.unwrap().is_success());
    }

    #[test]
    fn test_get_undoable_redoable_actions() {
        let snap_manager = SnapshotManager::new(10);
        let mut undo_manager = UndoManager::new(snap_manager);

        // Agregar 3 acciones
        for _ in 0..3 {
            let snap_id = undo_manager.create_snapshot(SnapshotReason::Checkpoint).unwrap();
            undo_manager.record_action(create_test_action(snap_id));
        }

        // Todas deshabibles, ninguna rehacer
        assert_eq!(undo_manager.get_undoable_actions().len(), 3);
        assert_eq!(undo_manager.get_redoable_actions().len(), 0);

        // Undo 2 veces
        undo_manager.confirm_undo();
        undo_manager.confirm_undo();

        // 1 deshacible, 2 rehacer
        assert_eq!(undo_manager.get_undoable_actions().len(), 1);
        assert_eq!(undo_manager.get_redoable_actions().len(), 2);
    }

    #[test]
    fn test_clear_history() {
        let snap_manager = SnapshotManager::new(10);
        let mut undo_manager = UndoManager::new(snap_manager);

        let snap_id = undo_manager.create_snapshot(SnapshotReason::Checkpoint).unwrap();
        undo_manager.record_action(create_test_action(snap_id));

        assert_eq!(undo_manager.history_count(), 1);

        undo_manager.clear_history();

        assert_eq!(undo_manager.history_count(), 0);
        assert_eq!(undo_manager.current_position(), 0);
    }
}
