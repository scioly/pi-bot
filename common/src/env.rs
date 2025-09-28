use std::{
    env, fs,
    io::{self, ErrorKind},
};

use log::{error, warn};
use serde;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum EnvError {
    #[error("io error: {0}")]
    Io(#[from] io::Error),
    #[error("envy error: {0}")]
    Envy(#[from] envy::Error),
}

pub fn load_env<T: serde::de::DeserializeOwned>() -> Result<T, EnvError> {
    let vars = env::vars();
    let secrets = fetch_secrets()?;
    let combined_vars = vars.into_iter().chain(secrets);
    Ok(envy::from_iter(combined_vars)?)
}

fn fetch_secrets() -> Result<Box<dyn Iterator<Item = (String, String)>>, io::Error> {
    let secret_dir = "/run/secrets/";
    let directory = match fs::read_dir(secret_dir) {
        Ok(secrets) => secrets,
        Err(err) => {
            return match err.kind() {
                ErrorKind::NotFound => {
                    warn!("`/run/secrets` was not found. No docker secrets will be loaded.");
                    Ok(Box::new([].into_iter()))
                }
                _ => Err(err),
            };
        }
    };
    Ok(Box::new(
        directory
            .into_iter()
            .filter_map(|res| match res {
                Ok(entry) => Some(entry),
                Err(e) => {
                    error!("Could not find secret file: {}", e);
                    None
                }
            })
            .filter_map(|entry| {
                let filename = entry.file_name();
                match fs::read_to_string(entry.path()) {
                    Ok(secret) => Some((filename.into_string().ok()?, secret.trim().to_string())),
                    Err(e) => {
                        error!("Could not load secret file: {}", e);
                        None
                    }
                }
            }),
    ))
}
