use std::{env, net::SocketAddr, path::PathBuf};

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub bind_addr: SocketAddr,
    pub agent_maker_home: PathBuf,
    pub serve_frontend_dist: Option<PathBuf>,
    /// Clerk JWKS endpoint (`/.well-known/jwks.json`). Auth is enforced only when
    /// this and `clerk_issuer` are both set; with both unset auth is disabled.
    pub clerk_jwks_url: Option<String>,
    /// Expected `iss` claim of Clerk session tokens.
    pub clerk_issuer: Option<String>,
    /// Optional cross-origin frontend origin to allow (CORS). Same-origin dev/prod
    /// needs no value.
    pub cors_allowed_origin: Option<String>,
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

        // Clerk auth is enforced only when both variables are present. Providing
        // exactly one is almost certainly a misconfiguration, so fail fast.
        let clerk_jwks_url = non_empty_var("CLERK_JWKS_URL");
        let clerk_issuer = non_empty_var("CLERK_ISSUER");
        if clerk_jwks_url.is_some() != clerk_issuer.is_some() {
            anyhow::bail!(
                "CLERK_JWKS_URL and CLERK_ISSUER must be set together (got only one); \
                 set both to enforce auth or neither to disable it"
            );
        }
        let cors_allowed_origin = non_empty_var("CORS_ALLOWED_ORIGIN");

        Ok(Self {
            database_url,
            bind_addr,
            agent_maker_home,
            serve_frontend_dist,
            clerk_jwks_url,
            clerk_issuer,
            cors_allowed_origin,
        })
    }
}

/// Read an env var, treating absent and empty-string as equivalently unset.
fn non_empty_var(key: &str) -> Option<String> {
    env::var(key).ok().map(|v| v.trim().to_string()).filter(|v| !v.is_empty())
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
