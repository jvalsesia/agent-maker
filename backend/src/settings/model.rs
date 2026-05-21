use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
pub struct SettingsDto {
    pub default_provider: String,
    pub default_model: DefaultModels,
    pub memory_defaults: MemoryDefaults,
    pub appearance: Appearance,
    pub providers: Vec<ProviderEntry>,
    pub key_store_backend: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DefaultModels {
    pub anthropic: Option<String>,
    pub openai: Option<String>,
    pub openai_compat: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MemoryDefaults {
    pub recent_n: i16,
    pub top_k: i16,
}

#[derive(Debug, Clone, Serialize)]
pub struct Appearance {
    pub theme: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProviderEntry {
    pub name: String,
    pub key_configured: bool,
    pub key_masked: Option<String>,
    pub base_url: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateSettings {
    #[serde(default)]
    pub default_provider: Option<String>,
    #[serde(default)]
    pub default_model: Option<DefaultModelsUpdate>,
    #[serde(default)]
    pub memory_defaults: Option<MemoryDefaultsUpdate>,
    #[serde(default)]
    pub appearance: Option<AppearanceUpdate>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DefaultModelsUpdate {
    #[serde(default)]
    pub anthropic: Option<String>,
    #[serde(default)]
    pub openai: Option<String>,
    #[serde(default)]
    pub openai_compat: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MemoryDefaultsUpdate {
    #[serde(default)]
    pub recent_n: Option<i16>,
    #[serde(default)]
    pub top_k: Option<i16>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AppearanceUpdate {
    #[serde(default)]
    pub theme: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PutKey {
    pub key: String,
    #[serde(default)]
    pub base_url: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WipeRequest {
    pub confirm: String,
}
