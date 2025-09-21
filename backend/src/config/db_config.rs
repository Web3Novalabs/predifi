use std::env;

#[derive(Debug, Clone)]
pub struct DbConfig {
    pub url: String,
    pub max_connections: u32,
}

impl DbConfig {
    pub fn from_env() -> Self {
        dotenv::dotenv().ok();
        Self {
            url: Self::get_env_var("DATABASE_URL"),
            max_connections: Self::get_env_var("DATABASE_MAX_CONNECTIONS")
                .parse()
                .unwrap_or(5),
        }
    }

    fn get_env_var(key: &str) -> String {
        env::var(key).unwrap_or_else(|_| panic!("{key} must be set"))
    }
}
