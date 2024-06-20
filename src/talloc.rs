use std::{
    fs::{self, File},
    io::{self, Write},
    path::Path,
};

use reqwest::blocking::Client;

use crate::errors::{Error, Result};

fn talloc_api_current_term_endpoint() -> &'static str {
    "https://cgi.cse.unsw.edu.au/~talloc/api/v1/term/current"
}

fn talloc_api_applications_endpoint(term_id: &str) -> String {
    format!(
        "https://cgi.cse.unsw.edu.au/~talloc/api/v1/terms/{}/applications",
        term_id
    )
}

fn make_request(client: &Client, endpoint: &str) -> Result<serde_json::Value> {
    let jwt = fs::read_to_string("jwt")
        .map_err(|err| Error::NoTallocJwt {
            error: err.to_string(),
        })?
        .trim()
        .to_string();
    if jwt.is_empty() {
        Err(Error::NoTallocJwt {
            error: "file is empty".into(),
        })?;
    }

    let response = client
        .get(endpoint)
        .header("x-jwt-auth", jwt)
        .header("Accept", "application/json")
        .send()
        .and_then(|response| response.error_for_status())
        .map_err(|err| Error::BadTallocResponse(err.to_string()))?;

    serde_json::from_reader(response).map_err(|err| {
        Box::new(Error::BadTallocResponse(format!(
            "could not decode talloc response as json: {err}"
        )))
    })
}

pub fn extract_talloc_term_id(term_info: serde_json::Value) -> Option<String> {
    let term_id = term_info.get("term_id")?;
    let term_name = term_info.get("term_name")?;
    println!("Using talloc application from term {term_name} (code {term_id})");
    Some(term_id.to_string())
}

pub fn fetch_applications_value(json_cache: &Path) -> Result<serde_json::Value> {
    if json_cache.exists() {
        let make_err = |err: String| {
            Box::new(Error::BadTallocCache {
                path: json_cache.to_string_lossy().into(),
                error: err.to_string(),
            })
        };

        println!("Using cached talloc download at {}", json_cache.display());

        let cache_file = File::open(json_cache)
            .map_err(|err| err.to_string())
            .map_err(make_err)?;
        serde_json::from_reader(cache_file)
            .map_err(|err| err.to_string())
            .map_err(make_err)
    } else {
        let client = reqwest::blocking::Client::new();

        let term_id =
            extract_talloc_term_id(make_request(&client, talloc_api_current_term_endpoint())?)
                .ok_or_else(|| {
                    Error::TallocParseFail("couldn't extract term_id/term_name".into())
                })?;

        print!("Downloading talloc applications, this may take a while... ");
        _ = io::stdout().flush();
        let applications = make_request(&client, &talloc_api_applications_endpoint(&term_id))?;
        println!("done!");

        fs::write(
            json_cache,
            serde_json::to_string(&applications)
                .expect("should be able to re-serialise what we just deserialised"),
        )
        .map_err(|err| Error::TallocCacheSaveFail(err.to_string()))?;
        println!("Cached download to {}", json_cache.display());

        Ok(applications)
    }
}
