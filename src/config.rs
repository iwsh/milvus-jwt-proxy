use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub listen_addr: String,
    pub upstream_url: String,
}

impl Config {
    pub fn from_env() -> Self {
        let listen_addr =
            env::var("PROXY_LISTEN_ADDR").unwrap_or_else(|_| "0.0.0.0:8000".to_string());
        let upstream_url =
            env::var("UPSTREAM_URL").unwrap_or_else(|_| "http://localhost:19530".to_string());

        Config {
            listen_addr,
            upstream_url,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_defaults() {
        // We can't safely use set_var/remove_var in multi-threaded tests in Rust 2024
        // without unsafe, and even then it's discouraged.
        // For these simple tests, we'll just check if the defaults are correct
        // when env vars are NOT set.
        let config = Config::from_env();
        // This test might fail if the environment already has these set.
        // In a real project we'd use a more robust config library.
        if env::var("PROXY_LISTEN_ADDR").is_err() {
            assert_eq!(config.listen_addr, "0.0.0.0:8000");
        }
        if env::var("UPSTREAM_URL").is_err() {
            assert_eq!(config.upstream_url, "http://localhost:19530");
        }
    }
}
