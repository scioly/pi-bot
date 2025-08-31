use std::{env, ffi::OsString, fs, io, rc::Rc};

use log::error;

struct EnvEntry {
    pub key: OsString,
    pub value: String,
}

/// This function should be called while still in single threaded mode
pub fn load_secrets() -> Result<(), io::Error> {
    let secrets = fetch_secrets()?;

    for entry in secrets.iter() {
        unsafe {
            env::set_var(&entry.key, &entry.value);
        }
    }

    Ok(())
}

fn fetch_secrets() -> Result<Rc<[EnvEntry]>, io::Error> {
    let secret_dir = "/run/secrets/";
    let directory = fs::read_dir(secret_dir)?;
    Ok(directory
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
                Ok(secret) => Some(EnvEntry {
                    key: filename,
                    value: secret.trim().to_string(),
                }),
                Err(e) => {
                    error!("Could not load secret file: {}", e);
                    None
                }
            }
        })
        .collect::<Rc<[_]>>())
}
