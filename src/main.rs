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

const APP_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), " ", env!("CARGO_PKG_VERSION"));

lazy_static::lazy_static! {
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

const HELP: &str = r#"
Yank a range of crate versions

USAGE:
    yanker "[0.1.0, 0.2.0]"
    yanker [OPTIONS]

OPTIONS:
    --version                Prints yanker's version
    --help                   Prints help information
"#;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        println!("{}", HELP);
        return Ok(());
    }

    let main_arg = args.pop().ok_or("")?;

    match main_arg.as_str() {
        "--help" | "-h" => {
            println!("{}", HELP);
            return Ok(());
        }
        "--version" | "-v" => {
            println!("{}", APP_USER_AGENT);
            return Ok(());
        }
        _ => (),
    }

    let (from, to) = {
        if RE.is_match(&main_arg) {
            let caps = RE.captures(&main_arg).ok_or("")?;
            let from = semver::Version::parse(caps.get(1).ok_or("")?.as_str())?;
            let to = semver::Version::parse(caps.get(2).ok_or("")?.as_str())?;
            (from, to)
        } else {
            (semver::Version::new(0, 0, 0), semver::Version::new(0, 0, 0))
        }
    };

    let local_toml = std::fs::read_to_string("Cargo.toml")?;
    let local_crate: Config = toml::from_str(&local_toml)?;

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
        tokio::process::Command::new("cargo")
            .args(&["yank", "--vers", &elem])
            .spawn()?
            .wait()
            .await?;
    }
    Ok(())
}
