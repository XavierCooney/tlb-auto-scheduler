use std::{io, num::ParseIntError, result};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("failed to read {tsv_path}: {io_error}")]
    FailedToReadTsv {
        tsv_path: String,
        io_error: io::Error,
    },
    #[error(
        "row {row:?} has {row_len} fields, but the header for TSV {path} has {header_len} fields"
    )]
    TsvNumFieldsMismatch {
        path: String,
        header_len: usize,
        row_len: usize,
        row: String,
    },
    #[error("the TSV '{tsv_path}' is missing the field {field:?}")]
    MissingTsvField { tsv_path: String, field: String },
    #[error("bad class requirement for {zid}'s {field}: {err}")]
    BadClassTypeRequirement {
        zid: String,
        field: String,
        err: ParseIntError,
    },
    #[error("bad boolean: {value:?}")]
    BadBoolean { value: String },
    #[error("problem with class {name}: {err}")]
    BadClass { name: String, err: String },
    #[error("error reading cached talloc at {path}: {error}")]
    BadTallocCache { path: String, error: String },
    #[error(
        "couldn't read talloc jwt: {error}\nCreate a file `jwt` with your talloc token from https://cgi.cse.unsw.edu.au/~talloc/admin/api "
    )]
    NoTallocJwt { error: String },
    #[error("failed to make talloc request: {0}")]
    BadTallocResponse(String),
    #[error("failed to save talloc cache: {0}")]
    TallocCacheSaveFail(String),
    #[error("failed to parse talloc response: {0}")]
    TallocParseFail(String),
}

pub type Result<T> = result::Result<T, Box<Error>>;
