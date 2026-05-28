use std::{env, net::SocketAddr, path::PathBuf};

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub bind_addr: SocketAddr,
    pub agent_maker_home: PathBuf,
    pub serve_frontend_dist: Option<PathBuf>,
    pub auth: AuthConfig,
}

#[derive(Debug, Clone)]
pub struct AuthConfig {
    pub fusionauth_base_url: String,
    pub fusionauth_tenant_id: String,
    pub fusionauth_application_id: String,
    pub fusionauth_client_id: String,
    pub fusionauth_client_secret: String,
    pub fusionauth_api_key: String,
    pub cookie_domain: Option<String>,
    pub cookie_secure: bool,
    pub disable_auth: bool,
    pub is_development: bool,
}

impl AuthConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        let is_development = env::var("RUST_ENV")
            .map(|v| v.eq_ignore_ascii_case("development"))
            .unwrap_or(false);
        let disable_auth = env::var("DISABLE_AUTH")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false)
            && is_development;

        let fusionauth_base_url = required_or_dev(
            "FUSIONAUTH_BASE_URL",
            disable_auth,
            "http://127.0.0.1:9011",
        )?;
        let fusionauth_tenant_id = required_or_dev(
            "FUSIONAUTH_TENANT_ID",
            disable_auth,
            "00000000-0000-0000-0000-000000003efa",
        )?;
        let fusionauth_application_id = required_or_dev(
            "FUSIONAUTH_APPLICATION_ID",
            disable_auth,
            "00000000-0000-0000-0000-0000000a9911",
        )?;
        let fusionauth_client_id = env::var("FUSIONAUTH_CLIENT_ID")
            .unwrap_or_else(|_| fusionauth_application_id.clone());
        let fusionauth_client_secret = required_or_dev(
            "FUSIONAUTH_CLIENT_SECRET",
            disable_auth,
            "agent-maker-local-dev-secret-do-not-use-in-prod",
        )?;
        let fusionauth_api_key = required_or_dev(
            "FUSIONAUTH_API_KEY",
            disable_auth,
            "agent-maker-local-dev-api-key-do-not-use-in-prod",
        )?;
        let cookie_domain = env::var("AUTH_COOKIE_DOMAIN")
            .ok()
            .filter(|v| !v.is_empty());
        let cookie_secure = env::var("AUTH_COOKIE_SECURE")
            .map(|v| !(v == "0" || v.eq_ignore_ascii_case("false")))
            .unwrap_or(true);

        Ok(Self {
            fusionauth_base_url,
            fusionauth_tenant_id,
            fusionauth_application_id,
            fusionauth_client_id,
            fusionauth_client_secret,
            fusionauth_api_key,
            cookie_domain,
            cookie_secure,
            disable_auth,
            is_development,
        })
    }
}

fn required_or_dev(var: &str, disable_auth: bool, dev_default: &str) -> anyhow::Result<String> {
    match env::var(var) {
        Ok(v) if !v.is_empty() => Ok(v),
        _ if disable_auth => Ok(dev_default.to_string()),
        _ => Err(anyhow::anyhow!(
            "{var} must be set (or run with RUST_ENV=development and DISABLE_AUTH=1 to skip)"
        )),
    }
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        let _ = dotenvy::dotenv();
        let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| {
            "postgres://agentmaker:agentmaker@127.0.0.1:5432/agentmaker".to_string()
        });
        // Prefer an explicit BIND_ADDR; otherwise fall back to the PORT variable
        // injected by platforms like Railway (binding all interfaces), and only
        // then to the local default.
        let bind_addr: SocketAddr = match env::var("BIND_ADDR") {
            Ok(addr) => addr.parse()?,
            Err(_) => match env::var("PORT") {
                Ok(port) => format!("0.0.0.0:{port}").parse()?,
                Err(_) => "127.0.0.1:8787".parse()?,
            },
        };
        let agent_maker_home = expand_home(
            &env::var("AGENT_MAKER_HOME").unwrap_or_else(|_| "~/.agent-maker".to_string()),
        );
        let serve_frontend_dist = if cfg!(debug_assertions) {
            None
        } else {
            Some(PathBuf::from("./frontend/dist"))
        };
        let auth = AuthConfig::from_env()?;
        Ok(Self {
            database_url,
            bind_addr,
            agent_maker_home,
            serve_frontend_dist,
            auth,
        })
    }
}

fn expand_home(input: &str) -> PathBuf {
    if let Some(rest) = input.strip_prefix("~/")
        && let Some(home) = dirs_home() {
            return home.join(rest);
        }
    PathBuf::from(input)
}

fn dirs_home() -> Option<PathBuf> {
    env::var_os("HOME").map(PathBuf::from)
}
