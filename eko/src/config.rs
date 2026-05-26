use std::{
    env,
    net::{AddrParseError, SocketAddr},
};

use tracing::warn;

pub struct Config {
    pub port: u16,
    pub domain: String,
    pub listen_addr: String,
    pub supabase_db_url: String,
    pub jwt_jwks_url: String,
}

pub fn get_from_env(var: &str, default: &str) -> String {
    env::var(var).unwrap_or_else(|_| {
        warn!("{} not set, using {}", var, default);
        default.to_string()
    })
}

impl Config {
    pub fn from_env() -> Self {
        let port_str = get_from_env("PORT", "3000");
        let port: u16 = port_str
            .parse()
            .expect("PORT environment variable must be a valid u16 integer");
        let domain = get_from_env("DOMAIN", &format!("http://127.0.0.1:{port}"));
        let listen_addr = get_from_env("LISTEN_ADDR", "0.0.0.0");
        let supabase_db_url = get_from_env(
            "SUPABASE_DB_URL",
            "postgresql://postgres:postgres@127.0.0.1:54322/postgres",
        );
        let jwt_jwks_url = env::var("JWT_JWKS_URL").expect("JWT_JWKS_URL must be set");
        Self {
            port,
            domain,
            listen_addr,
            supabase_db_url,
            jwt_jwks_url,
        }
    }

    pub fn get_addr(&self) -> Result<SocketAddr, AddrParseError> {
        let listen = &self.listen_addr;
        let port = self.port;
        format!("{listen}:{port}").parse()
    }
}
