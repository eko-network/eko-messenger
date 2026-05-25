use std::{
    env,
    net::{AddrParseError, SocketAddr},
};

pub struct Config {
    pub port: u16,
    pub domain: String,
    pub listen_addr: String,
}

impl Config {
    pub fn from_env() -> Self {
        let port_str = env::var("PORT").unwrap_or_else(|_| "3000".to_string());
        let port: u16 = port_str
            .parse()
            .expect("PORT environment variable must be a valid u16 integer");
        let domain = env::var("DOMAIN").unwrap_or_else(|_| format!("http://127.0.0.1:{port}"));
        let listen_addr = env::var("LISTEN_ADDR").unwrap_or_else(|_| "0.0.0.0".into());
        Self {
            port,
            domain,
            listen_addr,
        }
    }

    pub fn get_addr(&self) -> Result<SocketAddr, AddrParseError> {
        let listen = &self.listen_addr;
        let port = self.port;
        format!("{listen}:{port}").parse()
    }
}
