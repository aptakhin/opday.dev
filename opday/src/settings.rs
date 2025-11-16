use std::env;

pub struct Settings {
    pub local_run: bool,
    pub secret_key: String,
    pub postgres_dsn: String,
}

impl Settings {
    pub fn read_settings() -> Settings {
        let local_run: bool = env::var("OPDAY_LOCAL_RUN")
            .unwrap_or_default()
            .parse()
            .unwrap_or(false);
        // let secret_key =
        //     env::var("OPDAY_SECRET_KEY").expect("OPDAY_SECRET_KEY is expected for the run");
        let secret_key = "opday_dev_secret_key".to_string();
        // let postgres_dsn = Self::build_postgres_dsn();
        let postgres_dsn = "".to_string();
        Settings {
            local_run,
            secret_key,
            postgres_dsn,
        }
    }

    pub fn get_binary_secret_key(&self) -> &[u8] {
        self.secret_key.as_bytes()
    }
}
