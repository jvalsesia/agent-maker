use super::{SecretError, SecretStore};
use aes_gcm::{
    Aes256Gcm, KeyInit, Nonce,
    aead::Aead,
};
use argon2::Argon2;
use async_trait::async_trait;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Mutex,
};

const MASTER_FILE: &str = "master.key";
const SECRETS_FILE: &str = "secrets.bin";
const SALT_LEN: usize = 16;
const NONCE_LEN: usize = 12;

pub struct FileStore {
    home: PathBuf,
    cipher: Aes256Gcm,
    inner: Mutex<HashMap<String, String>>,
}

#[derive(Serialize, Deserialize, Default)]
struct Vault {
    entries: HashMap<String, String>,
}

impl FileStore {
    pub fn open_or_create(home: &Path) -> Result<Self, SecretError> {
        std::fs::create_dir_all(home).map_err(|e| SecretError::Backend(e.to_string()))?;
        let master = Self::load_or_create_master(&home.join(MASTER_FILE))?;
        let cipher = Aes256Gcm::new_from_slice(&master)
            .map_err(|e| SecretError::Backend(format!("cipher init: {e}")))?;
        let inner = Self::load_vault(&home.join(SECRETS_FILE), &cipher)?;
        Ok(Self {
            home: home.to_path_buf(),
            cipher,
            inner: Mutex::new(inner),
        })
    }

    fn load_or_create_master(path: &Path) -> Result<[u8; 32], SecretError> {
        if path.exists() {
            let raw = std::fs::read(path).map_err(|e| SecretError::Backend(e.to_string()))?;
            if raw.len() < SALT_LEN + 32 {
                return Err(SecretError::Backend("master.key corrupt".into()));
            }
            let salt = &raw[..SALT_LEN];
            let secret = &raw[SALT_LEN..SALT_LEN + 32];
            let mut key = [0u8; 32];
            Argon2::default()
                .hash_password_into(secret, salt, &mut key)
                .map_err(|e| SecretError::Backend(format!("argon2: {e}")))?;
            Ok(key)
        } else {
            let mut salt = [0u8; SALT_LEN];
            let mut secret = [0u8; 32];
            rand::thread_rng().fill_bytes(&mut salt);
            rand::thread_rng().fill_bytes(&mut secret);
            let mut buf = Vec::with_capacity(SALT_LEN + 32);
            buf.extend_from_slice(&salt);
            buf.extend_from_slice(&secret);
            std::fs::write(path, &buf).map_err(|e| SecretError::Backend(e.to_string()))?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = std::fs::metadata(path)
                    .map_err(|e| SecretError::Backend(e.to_string()))?
                    .permissions();
                perms.set_mode(0o600);
                std::fs::set_permissions(path, perms)
                    .map_err(|e| SecretError::Backend(e.to_string()))?;
            }
            let mut key = [0u8; 32];
            Argon2::default()
                .hash_password_into(&secret, &salt, &mut key)
                .map_err(|e| SecretError::Backend(format!("argon2: {e}")))?;
            Ok(key)
        }
    }

    fn load_vault(path: &Path, cipher: &Aes256Gcm) -> Result<HashMap<String, String>, SecretError> {
        if !path.exists() {
            return Ok(HashMap::new());
        }
        let raw = std::fs::read(path).map_err(|e| SecretError::Backend(e.to_string()))?;
        if raw.len() < NONCE_LEN {
            return Err(SecretError::Backend("secrets.bin corrupt".into()));
        }
        let (nonce_bytes, ct) = raw.split_at(NONCE_LEN);
        let plaintext = cipher
            .decrypt(Nonce::from_slice(nonce_bytes), ct)
            .map_err(|e| SecretError::Backend(format!("decrypt: {e}")))?;
        let vault: Vault = serde_json::from_slice(&plaintext)
            .map_err(|e| SecretError::Backend(format!("vault parse: {e}")))?;
        Ok(vault.entries)
    }

    fn persist(&self) -> Result<(), SecretError> {
        let entries = self.inner.lock().unwrap().clone();
        let vault = Vault { entries };
        let plaintext =
            serde_json::to_vec(&vault).map_err(|e| SecretError::Backend(e.to_string()))?;
        let mut nonce_bytes = [0u8; NONCE_LEN];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);
        let ct = self
            .cipher
            .encrypt(nonce, plaintext.as_ref())
            .map_err(|e| SecretError::Backend(format!("encrypt: {e}")))?;
        let mut out = Vec::with_capacity(NONCE_LEN + ct.len());
        out.extend_from_slice(&nonce_bytes);
        out.extend_from_slice(&ct);
        let path = self.home.join(SECRETS_FILE);
        std::fs::write(&path, &out).map_err(|e| SecretError::Backend(e.to_string()))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&path)
                .map_err(|e| SecretError::Backend(e.to_string()))?
                .permissions();
            perms.set_mode(0o600);
            std::fs::set_permissions(&path, perms)
                .map_err(|e| SecretError::Backend(e.to_string()))?;
        }
        Ok(())
    }
}

#[async_trait]
impl SecretStore for FileStore {
    async fn put(&self, name: &str, secret: &str) -> Result<(), SecretError> {
        self.inner.lock().unwrap().insert(name.to_string(), secret.to_string());
        self.persist()
    }
    async fn get(&self, name: &str) -> Result<String, SecretError> {
        self.inner
            .lock()
            .unwrap()
            .get(name)
            .cloned()
            .ok_or(SecretError::NotFound)
    }
    async fn delete(&self, name: &str) -> Result<(), SecretError> {
        self.inner.lock().unwrap().remove(name);
        self.persist()
    }
    async fn clear_all(&self, names: &[&str]) -> Result<(), SecretError> {
        {
            let mut g = self.inner.lock().unwrap();
            for n in names {
                g.remove(*n);
            }
        }
        self.persist()
    }
    fn backend_name(&self) -> &'static str {
        "file"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn round_trip_encrypt_decrypt() {
        let dir = tempdir().unwrap();
        let store = FileStore::open_or_create(dir.path()).unwrap();
        store.put("anthropic", "sk-ant-XYZ").await.unwrap();
        drop(store);
        let store2 = FileStore::open_or_create(dir.path()).unwrap();
        assert_eq!(store2.get("anthropic").await.unwrap(), "sk-ant-XYZ");
    }

    #[tokio::test]
    async fn missing_master_is_created_0600() {
        let dir = tempdir().unwrap();
        let _ = FileStore::open_or_create(dir.path()).unwrap();
        let mp = dir.path().join("master.key");
        assert!(mp.exists());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&mp).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode, 0o600);
        }
    }

    #[tokio::test]
    async fn corrupted_ciphertext_errors() {
        let dir = tempdir().unwrap();
        let store = FileStore::open_or_create(dir.path()).unwrap();
        store.put("openai", "sk-OAI").await.unwrap();
        let sp = dir.path().join("secrets.bin");
        let mut raw = std::fs::read(&sp).unwrap();
        // flip a byte beyond the nonce to corrupt the ciphertext authentication tag
        let last = raw.len() - 1;
        raw[last] ^= 0xFF;
        std::fs::write(&sp, &raw).unwrap();
        assert!(FileStore::open_or_create(dir.path()).is_err());
    }
}
