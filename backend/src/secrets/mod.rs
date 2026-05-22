use async_trait::async_trait;
use std::path::Path;

pub mod file_store;
pub mod keyring_store;

pub use file_store::FileStore;
pub use keyring_store::KeyringStore;

#[derive(Debug, thiserror::Error)]
pub enum SecretError {
    #[error("not found")]
    NotFound,
    #[error("backend error: {0}")]
    Backend(String),
}

#[async_trait]
pub trait SecretStore: Send + Sync {
    async fn put(&self, name: &str, secret: &str) -> Result<(), SecretError>;
    async fn get(&self, name: &str) -> Result<String, SecretError>;
    async fn delete(&self, name: &str) -> Result<(), SecretError>;
    async fn clear_all(&self, names: &[&str]) -> Result<(), SecretError>;
    fn backend_name(&self) -> &'static str;
}

pub enum AnyStore {
    Keyring(KeyringStore),
    File(FileStore),
}

#[async_trait]
impl SecretStore for AnyStore {
    async fn put(&self, name: &str, secret: &str) -> Result<(), SecretError> {
        match self {
            AnyStore::Keyring(s) => s.put(name, secret).await,
            AnyStore::File(s) => s.put(name, secret).await,
        }
    }
    async fn get(&self, name: &str) -> Result<String, SecretError> {
        match self {
            AnyStore::Keyring(s) => s.get(name).await,
            AnyStore::File(s) => s.get(name).await,
        }
    }
    async fn delete(&self, name: &str) -> Result<(), SecretError> {
        match self {
            AnyStore::Keyring(s) => s.delete(name).await,
            AnyStore::File(s) => s.delete(name).await,
        }
    }
    async fn clear_all(&self, names: &[&str]) -> Result<(), SecretError> {
        match self {
            AnyStore::Keyring(s) => s.clear_all(names).await,
            AnyStore::File(s) => s.clear_all(names).await,
        }
    }
    fn backend_name(&self) -> &'static str {
        match self {
            AnyStore::Keyring(s) => s.backend_name(),
            AnyStore::File(s) => s.backend_name(),
        }
    }
}

/// Prefers the OS keychain; falls back to a file-based encrypted store.
///
/// Set `AGENT_MAKER_FORCE_FILE_STORE=1` to skip the keyring probe and always use the
/// encrypted file store. Useful on Linux desktops where the Secret Service collection
/// is unreliable (locked between sessions, dropped on logout, etc.).
pub fn auto(home: &Path) -> AnyStore {
    if std::env::var("AGENT_MAKER_FORCE_FILE_STORE").ok().as_deref() == Some("1") {
        tracing::info!("secret store: file (forced via AGENT_MAKER_FORCE_FILE_STORE)");
        return AnyStore::File(FileStore::open_or_create(home).expect("init file secret store"));
    }
    match KeyringStore::new() {
        Ok(k) => {
            tracing::info!("secret store: keyring");
            AnyStore::Keyring(k)
        }
        Err(e) => {
            tracing::warn!(error = %e, "OS keychain unavailable, falling back to file store");
            AnyStore::File(FileStore::open_or_create(home).expect("init file secret store"))
        }
    }
}
