use super::model::*;
use crate::{
    error::{AppError, AppResult},
    llm::{ProviderName, provider::mask_key},
    secrets::{AnyStore, SecretError, SecretStore},
};
use sqlx::{PgPool, Row};
use std::sync::Arc;

const ALL_TABLES: &[&str] = &[
    "message_embeddings",
    "messages",
    "conversations",
    "agent_skills",
    "skills",
    "agents",
    "provider_keys",
];

const KEY_NAMES: &[&str] = &["anthropic", "openai", "openai_compat"];

#[derive(Clone)]
pub struct SettingsService {
    pub pool: PgPool,
    pub secrets: Arc<AnyStore>,
}

impl SettingsService {
    pub fn new(pool: PgPool, secrets: Arc<AnyStore>) -> Self {
        Self { pool, secrets }
    }

    pub async fn read(&self) -> AppResult<SettingsDto> {
        let row = sqlx::query(
            r#"SELECT default_provider, default_model_anthropic, default_model_openai,
                      default_model_openai_compat, recent_n, top_k, theme
               FROM settings WHERE id='singleton'"#,
        )
        .fetch_one(&self.pool)
        .await?;

        let key_rows = sqlx::query("SELECT name, key_masked, base_url FROM provider_keys")
            .fetch_all(&self.pool)
            .await?;

        let mut providers = Vec::with_capacity(3);
        for &p in &["anthropic", "openai", "openai_compat"] {
            let r = key_rows.iter().find(|r| {
                let n: String = r.get("name");
                n == p
            });
            providers.push(ProviderEntry {
                name: p.to_string(),
                key_configured: r.is_some(),
                key_masked: r.map(|r| r.get::<String, _>("key_masked")),
                base_url: r.and_then(|r| r.try_get::<Option<String>, _>("base_url").ok().flatten()),
            });
        }

        Ok(SettingsDto {
            default_provider: row.get("default_provider"),
            default_model: DefaultModels {
                anthropic: row.try_get("default_model_anthropic").ok(),
                openai: row.try_get("default_model_openai").ok(),
                openai_compat: row.try_get("default_model_openai_compat").ok(),
            },
            memory_defaults: MemoryDefaults {
                recent_n: row.get("recent_n"),
                top_k: row.get("top_k"),
            },
            appearance: Appearance { theme: row.get("theme") },
            providers,
            key_store_backend: self.secrets.backend_name().to_string(),
        })
    }

    pub async fn update(&self, patch: UpdateSettings) -> AppResult<SettingsDto> {
        if let Some(ref p) = patch.default_provider
            && p.parse::<ProviderName>().is_err() {
                return Err(AppError::validation_field(
                    "default_provider",
                    "must be one of anthropic, openai, openai_compat",
                ));
            }
        if let Some(ref m) = patch.memory_defaults {
            if let Some(n) = m.recent_n
                && !(4..=30).contains(&n) {
                    return Err(AppError::validation_field(
                        "memory_defaults.recent_n",
                        "must be in [4,30]",
                    ));
                }
            if let Some(k) = m.top_k
                && !(0..=10).contains(&k) {
                    return Err(AppError::validation_field(
                        "memory_defaults.top_k",
                        "must be in [0,10]",
                    ));
                }
        }
        if let Some(ref a) = patch.appearance
            && let Some(ref t) = a.theme
                && !matches!(t.as_str(), "light" | "dark" | "system") {
                    return Err(AppError::validation_field(
                        "appearance.theme",
                        "must be light, dark, or system",
                    ));
                }

        let mut tx = self.pool.begin().await?;
        if let Some(p) = patch.default_provider {
            sqlx::query("UPDATE settings SET default_provider=$1, updated_at=now() WHERE id='singleton'")
                .bind(p).execute(&mut *tx).await?;
        }
        if let Some(m) = patch.default_model {
            if let Some(v) = m.anthropic {
                sqlx::query("UPDATE settings SET default_model_anthropic=$1, updated_at=now() WHERE id='singleton'")
                    .bind(v).execute(&mut *tx).await?;
            }
            if let Some(v) = m.openai {
                sqlx::query("UPDATE settings SET default_model_openai=$1, updated_at=now() WHERE id='singleton'")
                    .bind(v).execute(&mut *tx).await?;
            }
            if let Some(v) = m.openai_compat {
                sqlx::query("UPDATE settings SET default_model_openai_compat=$1, updated_at=now() WHERE id='singleton'")
                    .bind(v).execute(&mut *tx).await?;
            }
        }
        if let Some(m) = patch.memory_defaults {
            if let Some(n) = m.recent_n {
                sqlx::query("UPDATE settings SET recent_n=$1, updated_at=now() WHERE id='singleton'")
                    .bind(n).execute(&mut *tx).await?;
            }
            if let Some(k) = m.top_k {
                sqlx::query("UPDATE settings SET top_k=$1, updated_at=now() WHERE id='singleton'")
                    .bind(k).execute(&mut *tx).await?;
            }
        }
        if let Some(a) = patch.appearance
            && let Some(t) = a.theme {
                sqlx::query("UPDATE settings SET theme=$1, updated_at=now() WHERE id='singleton'")
                    .bind(t).execute(&mut *tx).await?;
            }
        tx.commit().await?;
        self.read().await
    }

    pub async fn put_key(&self, name: ProviderName, req: PutKey) -> AppResult<()> {
        let key = req.key.trim().to_string();
        if key.is_empty() && !matches!(name, ProviderName::OpenAiCompat) {
            return Err(AppError::validation_field("key", "must be non-empty"));
        }
        let masked = mask_key(&key);
        if !key.is_empty() {
            self.secrets
                .put(name.as_str(), &key)
                .await
                .map_err(|e| AppError::KeyStore(e.to_string()))?;
        }
        sqlx::query(
            r#"INSERT INTO provider_keys (name, key_masked, base_url)
               VALUES ($1, $2, $3)
               ON CONFLICT (name) DO UPDATE
                 SET key_masked = EXCLUDED.key_masked,
                     base_url   = EXCLUDED.base_url,
                     updated_at = now()"#,
        )
        .bind(name.as_str())
        .bind(masked)
        .bind(req.base_url)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn delete_key(&self, name: ProviderName) -> AppResult<()> {
        match self.secrets.delete(name.as_str()).await {
            Ok(()) | Err(SecretError::NotFound) => {}
            Err(e) => return Err(AppError::KeyStore(e.to_string())),
        }
        sqlx::query("DELETE FROM provider_keys WHERE name=$1")
            .bind(name.as_str())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn base_url_for(&self, name: ProviderName) -> AppResult<Option<String>> {
        let row = sqlx::query("SELECT base_url FROM provider_keys WHERE name=$1")
            .bind(name.as_str())
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.and_then(|r| r.try_get::<Option<String>, _>("base_url").ok().flatten()))
    }

    pub async fn wipe(&self, req: WipeRequest) -> AppResult<()> {
        if req.confirm != "WIPE" {
            return Err(AppError::ConfirmationRequired);
        }
        tracing::warn!("wiping local data");
        let mut tx = self.pool.begin().await?;
        for table in ALL_TABLES {
            let sql = format!("TRUNCATE TABLE {table} RESTART IDENTITY CASCADE");
            sqlx::query(&sql).execute(&mut *tx).await?;
        }
        sqlx::query(
            r#"UPDATE settings
               SET default_provider='anthropic',
                   default_model_anthropic=NULL,
                   default_model_openai=NULL,
                   default_model_openai_compat=NULL,
                   recent_n=10, top_k=5, theme='system',
                   updated_at=now()
               WHERE id='singleton'"#,
        )
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        self.secrets
            .clear_all(KEY_NAMES)
            .await
            .map_err(|e| AppError::KeyStore(e.to_string()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn make_service() -> SettingsService {
        let pool = sqlx::PgPool::connect_lazy("postgres://x:y@127.0.0.1/none").unwrap();
        let store = Arc::new(crate::secrets::AnyStore::File(Box::new(
            crate::secrets::FileStore::open_or_create(tempfile::tempdir().unwrap().path()).unwrap(),
        )));
        SettingsService::new(pool, store)
    }

    #[tokio::test]
    async fn validation_rejects_bad_provider() {
        let svc = make_service().await;
        let patch = UpdateSettings {
            default_provider: Some("bogus".into()),
            ..Default::default()
        };
        assert!(matches!(svc.update(patch).await, Err(AppError::Validation { .. })));
    }

    #[tokio::test]
    async fn validation_rejects_recent_n_out_of_range() {
        let svc = make_service().await;
        let patch = UpdateSettings {
            memory_defaults: Some(MemoryDefaultsUpdate { recent_n: Some(2), top_k: None }),
            ..Default::default()
        };
        assert!(matches!(svc.update(patch).await, Err(AppError::Validation { .. })));
    }

    #[tokio::test]
    async fn validation_rejects_bad_theme() {
        let svc = make_service().await;
        let patch = UpdateSettings {
            appearance: Some(AppearanceUpdate { theme: Some("neon".into()) }),
            ..Default::default()
        };
        assert!(matches!(svc.update(patch).await, Err(AppError::Validation { .. })));
    }
}
