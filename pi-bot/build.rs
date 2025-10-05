use chrono::Utc;
use std::{process::Command, rc::Rc};

fn main() {
    println!("cargo:rerun-if-changed=migrations");
    println!("cargo:rerun-if-changed=.git/HEAD");
    const DEFAULT_VERSION: &str = "orphan";
    let git_hash = {
        let output = Command::new("git")
            .args(["rev-parse", "--short", "HEAD"])
            .output();
        if let Ok(output) = output {
            if let Ok(version_string) = String::from_utf8(output.stdout) {
                version_string.trim_end().to_string()
            } else {
                DEFAULT_VERSION.to_string()
            }
        } else {
            DEFAULT_VERSION.to_string()
        }
    };
    let profile = std::env::var("PROFILE").unwrap();
    let release = match profile.as_str() {
        "debug" => Some("dev"),
        "release" => None,
        _ => Some("unknown"),
    };
    let date = Utc::now().date_naive().format("%Y.%m.%d").to_string();
    let version_parts = &[Some(date.as_str()), Some(git_hash.as_str()), release]
        .iter()
        .filter_map(|&a| a)
        .collect::<Rc<[_]>>();
    println!("cargo:rustc-env=PI_BOT_VERSION={}", version_parts.join("-"));
}
