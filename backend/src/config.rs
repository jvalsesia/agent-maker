use std::{env, net::SocketAddr, path::PathBuf};

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub bind_addr: SocketAddr,
    pub agent_maker_home: PathBuf,
    pub serve_frontend_dist: Option<PathBuf>,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        let _ = dotenvy::dotenv();
        let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| {
            "postgres://agentmaker:agentmaker@127.0.0.1:5432/agentmaker".to_string()
        });
        let bind_addr: SocketAddr = env::var("BIND_ADDR")
            .unwrap_or_else(|_| "127.0.0.1:8787".to_string())
            .parse()?;
        let agent_maker_home = expand_home(
            &env::var("AGENT_MAKER_HOME").unwrap_or_else(|_| "~/.agent-maker".to_string()),
        );
        let serve_frontend_dist = if cfg!(debug_assertions) {
            None
        } else {
            Some(PathBuf::from("./frontend/dist"))
        };
        Ok(Self {
            database_url,
            bind_addr,
            agent_maker_home,
            serve_frontend_dist,
        })
    }
}

fn expand_home(input: &str) -> PathBuf {
    if let Some(rest) = input.strip_prefix("~/") {
        if let Some(home) = dirs_home() {
            return home.join(rest);
        }
    }
    PathBuf::from(input)
}

fn dirs_home() -> Option<PathBuf> {
    env::var_os("HOME").map(PathBuf::from)
}
