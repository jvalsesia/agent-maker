use super::{SecretError, SecretStore};
use async_trait::async_trait;

const SERVICE: &str = "agent-maker";

pub struct KeyringStore;

impl KeyringStore {
    pub fn new() -> Result<Self, SecretError> {
        // Probe by attempting to open an entry; if the platform has no backend, this fails.
        let entry = keyring::Entry::new(SERVICE, "__probe__")
            .map_err(|e| SecretError::Backend(e.to_string()))?;
        // A missing entry is fine; a backend-unavailable error is not.
        match entry.get_password() {
            Ok(_) | Err(keyring::Error::NoEntry) => Ok(Self),
            Err(e) => Err(SecretError::Backend(e.to_string())),
        }
    }
}

#[async_trait]
impl SecretStore for KeyringStore {
    async fn put(&self, name: &str, secret: &str) -> Result<(), SecretError> {
        let name_owned = name.to_string();
        let secret_owned = secret.to_string();
        let res = tokio::task::spawn_blocking(move || {
            keyring::Entry::new(SERVICE, &name_owned)
                .and_then(|e| e.set_password(&secret_owned))
                .map_err(|e| SecretError::Backend(e.to_string()))
        })
        .await
        .map_err(|e| SecretError::Backend(e.to_string()))?;
        match &res {
            Ok(()) => tracing::info!(provider = %name, "keyring put ok"),
            Err(e) => tracing::error!(provider = %name, error = %e, "keyring put failed"),
        }
        res
    }

    async fn get(&self, name: &str) -> Result<String, SecretError> {
        let name_owned = name.to_string();
        let res = tokio::task::spawn_blocking(move || {
            match keyring::Entry::new(SERVICE, &name_owned)
                .map_err(|e| SecretError::Backend(e.to_string()))?
                .get_password()
            {
                Ok(s) => Ok(s),
                Err(keyring::Error::NoEntry) => Err(SecretError::NotFound),
                Err(e) => Err(SecretError::Backend(e.to_string())),
            }
        })
        .await
        .map_err(|e| SecretError::Backend(e.to_string()))?;
        match &res {
            Ok(_) => tracing::debug!(provider = %name, "keyring get ok"),
            Err(SecretError::NotFound) => {
                tracing::warn!(provider = %name, "keyring get: entry not found")
            }
            Err(e) => tracing::error!(provider = %name, error = %e, "keyring get failed"),
        }
        res
    }

    async fn delete(&self, name: &str) -> Result<(), SecretError> {
        let name = name.to_string();
        tokio::task::spawn_blocking(move || {
            match keyring::Entry::new(SERVICE, &name)
                .map_err(|e| SecretError::Backend(e.to_string()))?
                .delete_credential()
            {
                Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
                Err(e) => Err(SecretError::Backend(e.to_string())),
            }
        })
        .await
        .map_err(|e| SecretError::Backend(e.to_string()))?
    }

    async fn clear_all(&self, names: &[&str]) -> Result<(), SecretError> {
        for n in names {
            self.delete(n).await?;
        }
        Ok(())
    }

    fn backend_name(&self) -> &'static str {
        "keyring"
    }
}
