use anyhow::Result;
use std::io;

use crate::hash_algorithms::HashAlgorithm;

pub fn validate_algorithms(algorithms: &[String]) -> Result<Vec<HashAlgorithm>> {
    algorithms
        .iter()
        .map(|algo| HashAlgorithm::new(algo))
        .collect()
}

#[derive(Debug)]
pub enum HashError {
    FileNotFound(io::Error),
    Other(anyhow::Error),
}

impl From<anyhow::Error> for HashError {
    fn from(err: anyhow::Error) -> Self {
        HashError::Other(err)
    }
}
