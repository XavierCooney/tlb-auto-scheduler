use std::{
    collections::HashMap,
    fs::{self, File},
    io::{self, Write},
    path::Path,
};

use anyhow::{anyhow, bail, Context, Result};
use reqwest::blocking::Client;

use crate::{
    classes::Mode,
    utils::{Day, TimeOfDay},
};

fn talloc_api_current_term_endpoint() -> &'static str {
    "https://talloc.cse.unsw.edu.au/api/v1/term/current"
}

fn talloc_api_applications_endpoint(term_id: &str) -> String {
    format!(
        "https://talloc.cse.unsw.edu.au/api/v1/terms/{}/applications",
        term_id
    )
}

fn read_jwt() -> Result<String> {
    let jwt = fs::read_to_string("jwt")
        .context("failed to read file `jwt` to get talloc token")?
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
            + "Hint: you should get a talloc token from\n"
            + "  https://cgi.cse.unsw.edu.au/~talloc/admin/api\nand put it in the "
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

    println!("Using talloc applications from term {term_name} (code {term_id})");
    Ok(term_id.to_string())
}

fn fetch_applications_value(json_cache: &Path) -> Result<serde_json::Value> {
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

fn group_talloc_by_applicant(
    raw_json: serde_json::Value,
) -> Result<HashMap<String, serde_json::Value>> {
    let applicants = match raw_json {
        serde_json::Value::Array(arr) => arr,
        _ => bail!("outer talloc JSON is not an array"),
    };

    applicants
        .into_iter()
        .map(|mut application| {
            let zid = application
                .pointer("/profile/zid")
                .with_context(|| anyhow!("application is missing a zid"))?
                .as_str()
                .context("profile.zid is not a string")?
                .to_string();
            Ok((
                zid.to_string(),
                application
                    .get_mut("application")
                    .with_context(|| anyhow!("{zid} does not have an associated application"))?
                    .take(),
            ))
        })
        .collect()
}

pub struct TallocApps {
    applications: HashMap<String, serde_json::Value>,
    ignore_no_application: bool,
}

impl TallocApps {
    pub fn fetch(json_cache: &Path, ignore_no_application: bool) -> Result<Self> {
        let raw_json = fetch_applications_value(json_cache)?;

        Ok(TallocApps {
            applications: group_talloc_by_applicant(raw_json).with_context(|| "bad talloc JSON")?,
            ignore_no_application,
        })
    }

    pub fn get_application<'a>(&'a self, zid: &str) -> Option<TallocApplication<'a>> {
        match self.applications.get(zid) {
            Some(application) => Some(TallocApplication::Application(application)),
            None => self
                .ignore_no_application
                .then_some(TallocApplication::NoApplication),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Availability {
    Impossible,
    Dislike,
    Possible,
    Preferred,
}

// #[derive(Clone, Copy)]
// pub struct TallocApplication<'a> {
//     application: &'a serde_json::Value,
// }

#[derive(Clone, Copy)]
pub enum TallocApplication<'a> {
    Application(&'a serde_json::Value),
    NoApplication,
}

impl TallocApplication<'_> {
    pub fn get_availability(&self, day: Day, time: TimeOfDay, mode: Mode) -> Option<Availability> {
        let availability_key = format!("{}{:02}", day.short_lowercase(), time.as_24_hours());

        let application = match self {
            TallocApplication::Application(application) => application,
            TallocApplication::NoApplication => return Some(Availability::Impossible),
        };

        let mut raw_availability = application
            .get(availability_key)?
            .as_str()?
            .parse::<u8>()
            .ok()?;

        if mode == Mode::Online {
            raw_availability >>= 2;
        }

        Some(match raw_availability & 0b11 {
            0 => Availability::Impossible,
            1 => Availability::Dislike,
            2 => Availability::Possible,
            3 => Availability::Preferred,
            _ => return None,
        })
    }

    pub fn is_default(&self) -> bool {
        match self {
            TallocApplication::Application(_) => false,
            TallocApplication::NoApplication => true,
        }
    }
}
