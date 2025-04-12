use miette::Report;
use miette::Result as R;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Parameters {
    pub repository_path: String,
    pub repository_url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Result {}

pub struct Error {
    pub base: Report,
}
trait HasReport {
    fn base(&self) -> &Report;
}

impl HasReport for Error {
    fn base(&self) -> &Report {
        &self.base
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ListError: {}", self.base)
    }
}

//////////////////////////////////////////////////////////////////////////////
pub fn run(_: &Parameters) -> R<Result, Error> {
    // Implementation of the function
    Ok(Result {})
}
//////////////////////////////////////////////////////////////////////////////
