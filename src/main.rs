//! # yanker
//!
//! A Rust crate to automate yanking crates.
//!
//! ## Usage
//! Install via cargo-install:
//! ```ignored
//! $ cargo install yanker
//! ```
//!
//! Change directories to the crate you want to yank:
//! ```ignored
//! $ cd path/to/crate/repo
//! $ yanker "[0.1.1, 0.2.5]"
//! ```
//! Should yank all versions between 0.1.1 (included) to 0.2.5 (excluded).
//!

#[macro_use]
extern crate lazy_static;
extern crate regex;
extern crate semver;
extern crate serde;
extern crate tokio;
extern crate toml;

use std::env;
use std::error;
use std::fs;
use std::process;

static APP_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"),);

lazy_static! {
    static ref RE: regex::Regex = regex::Regex::new(r"\[(.*), *(.*)\]").unwrap();
}

#[derive(Debug, serde::Deserialize)]
struct Package {
    name: String,
}

#[derive(Debug, serde::Deserialize)]
struct Config {
    package: Package,
}

#[derive(Debug, serde::Deserialize)]
struct Version {
    #[serde(rename = "crate")]
    crate_name: String,
    #[serde(rename = "num")]
    version: String,
    yanked: bool,
}

#[derive(Debug, serde::Deserialize)]
struct Versions {
    versions: Vec<Version>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 || args[1] == "--help" {
        println!("Usage: yanker \"[0.1.0, 0.2.0]\"");
        return Ok(());
    }

    let local_toml = fs::read_to_string("Cargo.toml")?;
    let local_crate: Config = toml::from_str(&local_toml)?;

    let (from, to) = {
        if RE.is_match(&args[1]) {
            let caps = RE.captures(&args[1]).ok_or("")?;
            let from = semver::Version::parse(caps.get(1).ok_or("")?.as_str())?;
            let to = semver::Version::parse(caps.get(2).ok_or("")?.as_str())?;
            (from, to)
        } else {
            (semver::Version::new(0, 0, 0), semver::Version::new(0, 0, 0))
        }
    };

    let client = reqwest::Client::builder()
        .user_agent(APP_USER_AGENT)
        .build()?;
    let resp = client
        .get(&format!(
            "https://crates.io/api/v1/crates/{}/versions",
            local_crate.package.name
        ))
        .send()
        .await?
        .json::<Versions>()
        .await?;

    let v: Vec<String> = resp
        .versions
        .iter()
        .filter(|item| {
            let yanked = item.yanked;
            if let Ok(item) = semver::Version::parse(&item.version) {
                item >= from && item < to && !yanked
            } else {
                false
            }
        })
        .map(|item| item.version.clone())
        .collect();

    for elem in v {
        process::Command::new("cargo")
            .args(&["yank", "--vers", &elem])
            .spawn()?;
    }
    Ok(())
}
