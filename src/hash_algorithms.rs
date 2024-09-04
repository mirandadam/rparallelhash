use anyhow::{anyhow, Result};
use digest::Digest;
use md5::Md5;
use sha1::Sha1;
use sha2::{Sha256, Sha384, Sha512};
use sha3::{Sha3_256, Sha3_384, Sha3_512};

#[derive(Clone, Debug)]
pub enum HashAlgorithm {
    Md5(Md5),
    Sha1(Sha1),
    Sha256(Sha256),
    Sha384(Sha384),
    Sha512(Sha512),
    Sha3_256(Sha3_256),
    Sha3_384(Sha3_384),
    Sha3_512(Sha3_512),
}

impl HashAlgorithm {
    pub fn new(algo: &str) -> Result<Self> {
        match algo.to_lowercase().as_str() {
            "md5" => Ok(HashAlgorithm::Md5(Md5::new())),
            "sha1" => Ok(HashAlgorithm::Sha1(Sha1::new())),
            "sha256" => Ok(HashAlgorithm::Sha256(Sha256::new())),
            "sha384" => Ok(HashAlgorithm::Sha384(Sha384::new())),
            "sha512" => Ok(HashAlgorithm::Sha512(Sha512::new())),
            "sha3-256" => Ok(HashAlgorithm::Sha3_256(Sha3_256::new())),
            "sha3-384" => Ok(HashAlgorithm::Sha3_384(Sha3_384::new())),
            "sha3-512" => Ok(HashAlgorithm::Sha3_512(Sha3_512::new())),
            _ => Err(anyhow!("Unsupported algorithm: {}", algo)),
        }
    }

    pub fn update(&mut self, data: &[u8]) {
        match self {
            HashAlgorithm::Md5(h) => h.update(data),
            HashAlgorithm::Sha1(h) => h.update(data),
            HashAlgorithm::Sha256(h) => h.update(data),
            HashAlgorithm::Sha384(h) => h.update(data),
            HashAlgorithm::Sha512(h) => h.update(data),
            HashAlgorithm::Sha3_256(h) => h.update(data),
            HashAlgorithm::Sha3_384(h) => h.update(data),
            HashAlgorithm::Sha3_512(h) => h.update(data),
        }
    }

    pub fn finalize_reset(&mut self) -> Vec<u8> {
        match self {
            HashAlgorithm::Md5(h) => h.finalize_reset().to_vec(),
            HashAlgorithm::Sha1(h) => h.finalize_reset().to_vec(),
            HashAlgorithm::Sha256(h) => h.finalize_reset().to_vec(),
            HashAlgorithm::Sha384(h) => h.finalize_reset().to_vec(),
            HashAlgorithm::Sha512(h) => h.finalize_reset().to_vec(),
            HashAlgorithm::Sha3_256(h) => h.finalize_reset().to_vec(),
            HashAlgorithm::Sha3_384(h) => h.finalize_reset().to_vec(),
            HashAlgorithm::Sha3_512(h) => h.finalize_reset().to_vec(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct FileChunk {
    pub data: Vec<u8>,
    pub is_last: bool,
}
