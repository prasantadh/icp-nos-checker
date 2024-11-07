use crate::{Error, Result};

use core::panic;
use std::{env, sync::OnceLock};

fn get_env(name: &'static str) -> Result<String> {
    env::var(name).map_err(|e| Error::VarError(e, name))
}

#[derive(Debug, Clone)]
pub struct Config {
    pub assignments: String,
    pub submissions: String,
    pub downloads: String,
    pub password: String,
    pub jwt_key: String,
}

impl Config {
    fn load_from_env() -> Result<Config> {
        Ok(Config {
            assignments: get_env("ASSIGNMENTS")?,
            submissions: get_env("SUBMISSIONS")?,
            downloads: get_env("DOWNLOADS")?,
            password: get_env("PASSWORD")?,
            jwt_key: get_env("JWT_KEY")?,
        })
    }
}

pub fn config() -> &'static Config {
    static INSTANCE: OnceLock<Config> = OnceLock::new();
    INSTANCE.get_or_init(|| match Config::load_from_env() {
        Ok(config) => config,
        Err(e) => panic!("FATAL - while loading config - {e:?}"),
    })
}
