// src/agent/snapshot.rs
// Sistema de snapshots para self-healing seguro

use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};

/// Identificador único de snapshot
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SnapshotId(pub String);

impl SnapshotId {
    pub fn new() -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        Self(format!("snap_{}", timestamp))
    }

    pub fn from_string(s: String) -> Self {
        Self(s)
    }
}

impl Default for SnapshotId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for SnapshotId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Razón por la que se creó el snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SnapshotReason {
    /// Antes de aplicar un fix de healing
    BeforeHeal { error_id: String },
    /// Antes de hot reload
    BeforeHotReload,
    /// Snapshot manual del usuario
    Manual { description: String },
    /// Checkpoint automático
    Checkpoint,
}

impl std::fmt::Display for SnapshotReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BeforeHeal { error_id } => write!(f, "Before healing error: {}", error_id),
            Self::BeforeHotReload => write!(f, "Before hot reload"),
            Self::Manual { description } => write!(f, "Manual: {}", description),
            Self::Checkpoint => write!(f, "Automatic checkpoint"),
        }
    }
}

/// Snapshot de un archivo individual
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileSnapshot {
    pub path: PathBuf,
    pub content: String,
    pub hash: String,
}

impl FileSnapshot {
    pub fn new(path: PathBuf, content: String) -> Self {
        let hash = Self::compute_hash(&content);
        Self { path, content, hash }
    }

    fn compute_hash(content: &str) -> String {
        // Simple hash para comparación rápida
        let mut hash: u64 = 0;
        for byte in content.bytes() {
            hash = hash.wrapping_mul(31).wrapping_add(byte as u64);
        }
        format!("{:016x}", hash)
    }

    pub fn content_changed(&self, new_content: &str) -> bool {
        Self::compute_hash(new_content) != self.hash
    }
}

/// Un snapshot completo del estado
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub id: SnapshotId,
    pub timestamp: u64,
    pub files: HashMap<PathBuf, FileSnapshot>,
    pub reason: SnapshotReason,
}

impl Snapshot {
    pub fn new(reason: SnapshotReason) -> Self {
        Self {
            id: SnapshotId::new(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            files: HashMap::new(),
            reason,
        }
    }

    pub fn add_file(&mut self, path: PathBuf, content: String) {
        self.files.insert(path.clone(), FileSnapshot::new(path, content));
    }

    pub fn get_file(&self, path: &PathBuf) -> Option<&FileSnapshot> {
        self.files.get(path)
    }
}

/// Resumen de un snapshot para listados
#[derive(Debug, Clone)]
pub struct SnapshotSummary {
    pub id: SnapshotId,
    pub timestamp: u64,
    pub reason: String,
    pub file_count: usize,
}

impl From<&Snapshot> for SnapshotSummary {
    fn from(snap: &Snapshot) -> Self {
        Self {
            id: snap.id.clone(),
            timestamp: snap.timestamp,
            reason: snap.reason.to_string(),
            file_count: snap.files.len(),
        }
    }
}

/// Resultado de restaurar un snapshot
#[derive(Debug)]
pub struct RestoreResult {
    pub snapshot_id: SnapshotId,
    pub files_restored: Vec<PathBuf>,
    pub files_failed: Vec<(PathBuf, String)>,
}

impl RestoreResult {
    pub fn is_success(&self) -> bool {
        self.files_failed.is_empty()
    }
}

/// Errores del sistema de snapshots
#[derive(Debug)]
pub enum SnapshotError {
    SnapshotNotFound(SnapshotId),
    IoError(String),
    SerializationError(String),
    MaxSnapshotsReached,
}

impl std::fmt::Display for SnapshotError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SnapshotNotFound(id) => write!(f, "Snapshot not found: {}", id),
            Self::IoError(msg) => write!(f, "IO error: {}", msg),
            Self::SerializationError(msg) => write!(f, "Serialization error: {}", msg),
            Self::MaxSnapshotsReached => write!(f, "Maximum snapshots reached"),
        }
    }
}

impl std::error::Error for SnapshotError {}

/// Gestor de snapshots
pub struct SnapshotManager {
    snapshots: VecDeque<Snapshot>,
    max_snapshots: usize,
    /// Si true, persiste snapshots a disco
    persist: bool,
    storage_path: Option<PathBuf>,
}

impl SnapshotManager {
    pub fn new(max_snapshots: usize) -> Self {
        Self {
            snapshots: VecDeque::new(),
            max_snapshots,
            persist: false,
            storage_path: None,
        }
    }

    pub fn with_persistence(mut self, path: PathBuf) -> Self {
        self.persist = true;
        self.storage_path = Some(path);
        self
    }

    /// Crea un nuevo snapshot
    pub fn create_snapshot(&mut self, reason: SnapshotReason) -> Result<SnapshotId, SnapshotError> {
        let snapshot = Snapshot::new(reason);
        let id = snapshot.id.clone();

        // Limitar número de snapshots
        while self.snapshots.len() >= self.max_snapshots {
            self.snapshots.pop_front();
        }

        self.snapshots.push_back(snapshot);
        Ok(id)
    }

    /// Crea un snapshot con archivos
    pub fn create_snapshot_with_files(
        &mut self,
        reason: SnapshotReason,
        files: Vec<(PathBuf, String)>,
    ) -> Result<SnapshotId, SnapshotError> {
        let mut snapshot = Snapshot::new(reason);

        for (path, content) in files {
            snapshot.add_file(path, content);
        }

        let id = snapshot.id.clone();

        while self.snapshots.len() >= self.max_snapshots {
            self.snapshots.pop_front();
        }

        self.snapshots.push_back(snapshot);
        Ok(id)
    }

    /// Obtiene un snapshot por ID
    pub fn get_snapshot(&self, id: &SnapshotId) -> Option<&Snapshot> {
        self.snapshots.iter().find(|s| s.id == *id)
    }

    /// Lista todos los snapshots
    pub fn list_snapshots(&self) -> Vec<SnapshotSummary> {
        self.snapshots.iter().map(SnapshotSummary::from).collect()
    }

    /// Restaura un snapshot (devuelve los contenidos a restaurar)
    pub fn get_restore_data(&self, id: &SnapshotId) -> Result<&Snapshot, SnapshotError> {
        self.get_snapshot(id)
            .ok_or_else(|| SnapshotError::SnapshotNotFound(id.clone()))
    }

    /// Elimina snapshots antiguos, manteniendo los N más recientes
    pub fn prune(&mut self, keep: usize) -> usize {
        let to_remove = self.snapshots.len().saturating_sub(keep);
        for _ in 0..to_remove {
            self.snapshots.pop_front();
        }
        to_remove
    }

    /// Número de snapshots almacenados
    pub fn count(&self) -> usize {
        self.snapshots.len()
    }

    /// Obtiene el snapshot más reciente
    pub fn latest(&self) -> Option<&Snapshot> {
        self.snapshots.back()
    }

    /// Elimina un snapshot específico
    pub fn remove(&mut self, id: &SnapshotId) -> bool {
        if let Some(pos) = self.snapshots.iter().position(|s| s.id == *id) {
            self.snapshots.remove(pos);
            true
        } else {
            false
        }
    }
}

impl Default for SnapshotManager {
    fn default() -> Self {
        Self::new(50) // 50 snapshots por defecto
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snapshot_id_unique() {
        let id1 = SnapshotId::new();
        std::thread::sleep(std::time::Duration::from_nanos(1));
        let id2 = SnapshotId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_file_snapshot_hash() {
        let snap1 = FileSnapshot::new(PathBuf::from("test.aura"), "content".to_string());
        let snap2 = FileSnapshot::new(PathBuf::from("test.aura"), "content".to_string());
        let snap3 = FileSnapshot::new(PathBuf::from("test.aura"), "different".to_string());

        assert_eq!(snap1.hash, snap2.hash);
        assert_ne!(snap1.hash, snap3.hash);
    }

    #[test]
    fn test_file_snapshot_content_changed() {
        let snap = FileSnapshot::new(PathBuf::from("test.aura"), "original".to_string());

        assert!(!snap.content_changed("original"));
        assert!(snap.content_changed("modified"));
    }

    #[test]
    fn test_snapshot_manager_create() {
        let mut manager = SnapshotManager::new(10);
        let id = manager.create_snapshot(SnapshotReason::Checkpoint).unwrap();

        assert_eq!(manager.count(), 1);
        assert!(manager.get_snapshot(&id).is_some());
    }

    #[test]
    fn test_snapshot_manager_max_limit() {
        let mut manager = SnapshotManager::new(3);

        let _id1 = manager.create_snapshot(SnapshotReason::Checkpoint).unwrap();
        let _id2 = manager.create_snapshot(SnapshotReason::Checkpoint).unwrap();
        let id3 = manager.create_snapshot(SnapshotReason::Checkpoint).unwrap();
        let id4 = manager.create_snapshot(SnapshotReason::Checkpoint).unwrap();

        assert_eq!(manager.count(), 3);
        // id1 should be removed (oldest)
        assert!(manager.get_snapshot(&id3).is_some());
        assert!(manager.get_snapshot(&id4).is_some());
    }

    #[test]
    fn test_snapshot_with_files() {
        let mut manager = SnapshotManager::new(10);
        let files = vec![
            (PathBuf::from("main.aura"), "main code".to_string()),
            (PathBuf::from("lib.aura"), "lib code".to_string()),
        ];

        let id = manager
            .create_snapshot_with_files(SnapshotReason::BeforeHotReload, files)
            .unwrap();

        let snapshot = manager.get_snapshot(&id).unwrap();
        assert_eq!(snapshot.files.len(), 2);
        assert_eq!(
            snapshot.get_file(&PathBuf::from("main.aura")).unwrap().content,
            "main code"
        );
    }

    #[test]
    fn test_snapshot_manager_prune() {
        let mut manager = SnapshotManager::new(10);

        for _ in 0..5 {
            manager.create_snapshot(SnapshotReason::Checkpoint).unwrap();
        }

        assert_eq!(manager.count(), 5);
        let removed = manager.prune(2);
        assert_eq!(removed, 3);
        assert_eq!(manager.count(), 2);
    }

    #[test]
    fn test_snapshot_manager_remove() {
        let mut manager = SnapshotManager::new(10);

        let id1 = manager.create_snapshot(SnapshotReason::Checkpoint).unwrap();
        let id2 = manager.create_snapshot(SnapshotReason::Checkpoint).unwrap();

        assert!(manager.remove(&id1));
        assert_eq!(manager.count(), 1);
        assert!(manager.get_snapshot(&id2).is_some());
        assert!(manager.get_snapshot(&id1).is_none());
    }

    #[test]
    fn test_snapshot_manager_latest() {
        let mut manager = SnapshotManager::new(10);

        assert!(manager.latest().is_none());

        let _id1 = manager.create_snapshot(SnapshotReason::Checkpoint).unwrap();
        let id2 = manager.create_snapshot(SnapshotReason::BeforeHotReload).unwrap();

        assert_eq!(manager.latest().unwrap().id, id2);
    }

    #[test]
    fn test_snapshot_reason_display() {
        let reason1 = SnapshotReason::BeforeHeal {
            error_id: "err_123".to_string(),
        };
        assert!(reason1.to_string().contains("err_123"));

        let reason2 = SnapshotReason::Manual {
            description: "test snapshot".to_string(),
        };
        assert!(reason2.to_string().contains("test snapshot"));
    }

    #[test]
    fn test_list_snapshots() {
        let mut manager = SnapshotManager::new(10);

        manager.create_snapshot(SnapshotReason::Checkpoint).unwrap();
        manager
            .create_snapshot(SnapshotReason::BeforeHotReload)
            .unwrap();

        let summaries = manager.list_snapshots();
        assert_eq!(summaries.len(), 2);
    }
}
