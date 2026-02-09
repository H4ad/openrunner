use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Group not found: {0}")]
    GroupNotFound(String),

    #[error("Project not found: {0}")]
    ProjectNotFound(String),

    #[error("Process already running for project: {0}")]
    ProcessAlreadyRunning(String),

    #[error("No process running for project: {0}")]
    ProcessNotRunning(String),

    #[error("Failed to spawn process: {0}")]
    SpawnError(String),

    #[error("Storage error: {0}")]
    StorageError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("PTY error: {0}")]
    PtyError(String),

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("File not found: {0}")]
    FileNotFound(String),
}

impl Serialize for Error {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
