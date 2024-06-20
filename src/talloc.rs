use std::{
    fs::{self, File},
    io::{self, Write},
    path::Path,
};

use anyhow::{anyhow, bail, Context, Result};
use reqwest::blocking::Client;

fn talloc_api_current_term_endpoint() -> &'static str {
    "https://cgi.cse.unsw.edu.au/~talloc/api/v1/term/current"
}

fn talloc_api_applications_endpoint(term_id: &str) -> String {
    format!(
        "https://cgi.cse.unsw.edu.au/~talloc/api/v1/terms/{}/applications",
        term_id
    )
}

fn read_jwt() -> Result<String> {
    let jwt = fs::read_to_string("jwt")
        .context("failed to read file `jwt` to get talloc ")?
        .trim()
        .to_string();
    if jwt.is_empty() {
        bail!("jwt file is empty")
    }
    Ok(jwt)
}

fn make_request(client: &Client, endpoint: &str) -> Result<serde_json::Value> {
    let jwt = read_jwt().with_context(|| {
        "could not get JWT for talloc auth.\n".to_string()
            + "you should get a talloc token from "
            + "https://cgi.cse.unsw.edu.au/~talloc/admin/api and put it in the "
            + "file `jwt` in your current working directory."
    })?;

    let response = client
        .get(endpoint)
        .header("x-jwt-auth", jwt)
        .header("Accept", "application/json")
        .send()
        .and_then(|response| response.error_for_status())
        .with_context(|| anyhow!("failed to fetch {endpoint}"))?;

    serde_json::from_reader(response).context("failed to decode talloc response as json")
}

pub fn extract_talloc_term_id(term_info: serde_json::Value) -> Result<String> {
    let term_id = term_info
        .get("term_id")
        .context("couldn't extract term_id from term info")?;
    let term_name = term_info
        .get("term_name")
        .context("couldn't extract term_name from term info")?;

    println!("Using talloc application from term {term_name} (code {term_id})");
    Ok(term_id.to_string())
}

pub fn fetch_applications_value(json_cache: &Path) -> Result<serde_json::Value> {
    if json_cache.exists() {
        println!("Using cached talloc download at {}", json_cache.display());

        let cache_file = File::open(json_cache).with_context(|| {
            anyhow!(
                "failed to read cache of talloc applications at {}",
                json_cache.display()
            )
        })?;
        serde_json::from_reader(cache_file).with_context(|| {
            anyhow!(
                "failed to parse cache of talloc applications at {}",
                json_cache.display()
            )
        })
    } else {
        let client = reqwest::blocking::Client::new();

        let term_id = extract_talloc_term_id(
            make_request(&client, talloc_api_current_term_endpoint())
                .context("failed to fetch term_info")?,
        )?;

        print!("Downloading talloc applications, this may take a while... ");
        _ = io::stdout().flush();
        let applications = make_request(&client, &talloc_api_applications_endpoint(&term_id))?;
        println!("done!");

        fs::write(
            json_cache,
            serde_json::to_string(&applications)
                .expect("should be able to re-serialise what we just deserialised"),
        )
        .with_context(|| {
            anyhow!(
                "failed to write cache of talloc download at {}",
                json_cache.display()
            )
        })?;
        println!("Cached download to {}", json_cache.display());

        Ok(applications)
    }
}
