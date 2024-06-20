use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use classes::Class;
use instructor::Instructor;
use session::{classes_to_sessions, OverlapMatrix, OverlapRequirement};
use talloc::fetch_applications_value;
use tsv::Tsv;

mod classes;
mod instructor;
mod session;
mod talloc;
mod tsv;
mod utils;

#[derive(Debug, clap::Parser)]
struct Args {
    config_dir: PathBuf,
}

impl Args {
    fn get_file_path(&self, filename: &str) -> PathBuf {
        self.config_dir.join(filename)
    }
}

fn main_impl() -> Result<()> {
    let args = Args::parse();

    let instructors = Instructor::vec_from_tsv(&Tsv::read_from_path(
        &args.get_file_path("instructors.tsv"),
    )?)?;
    println!("{instructors:#?}");

    let classes = Class::vec_from_tsv(&Tsv::read_from_path(&args.get_file_path("classes.tsv"))?)?;
    println!("{classes:#?}");

    let sessions = classes_to_sessions(&classes);
    println!("{sessions:#?}");

    let overlaps = OverlapMatrix::from_sessions(&sessions, OverlapRequirement::Sharp);
    println!("Overlaps:\n{}", overlaps.summarise(&sessions));

    fetch_applications_value(&args.get_file_path("talloc_cache.json"))?;

    todo!()
}

fn main() {
    match main_impl() {
        Ok(_) => {}
        Err(err) => println!("\nError: {:?}", err),
    }
}
