use anyhow::{anyhow, Result};
use blake3::Hasher as Blake3;
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
    Blake3(Blake3),
}

impl HashAlgorithm {
    pub fn new(algo: &str) -> Result<Self> {
        match algo.to_lowercase().as_str() {
            "md5" => Ok(HashAlgorithm::Md5(Md5::new())),
            "sha1" => Ok(HashAlgorithm::Sha1(Sha1::new())),
            "sha256" | "sha2-256" => Ok(HashAlgorithm::Sha256(Sha256::new())),
            "sha384" | "sha2-384" => Ok(HashAlgorithm::Sha384(Sha384::new())),
            "sha512" | "sha2-512" => Ok(HashAlgorithm::Sha512(Sha512::new())),
            "sha3-256" => Ok(HashAlgorithm::Sha3_256(Sha3_256::new())),
            "sha3-384" => Ok(HashAlgorithm::Sha3_384(Sha3_384::new())),
            "sha3-512" => Ok(HashAlgorithm::Sha3_512(Sha3_512::new())),
            "blake3" => Ok(HashAlgorithm::Blake3(Blake3::new())),
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
            HashAlgorithm::Blake3(h) => {
                h.update_rayon(data);
            }
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
            HashAlgorithm::Blake3(h) => {
                let result = h.finalize().as_bytes().to_vec();
                *h = Blake3::new();
                result
            }
        }
    }

    pub fn to_string(&self) -> String {
        match self {
            HashAlgorithm::Md5(_) => "MD5".to_string(),
            HashAlgorithm::Sha1(_) => "SHA1".to_string(),
            HashAlgorithm::Sha256(_) => "SHA2-256".to_string(),
            HashAlgorithm::Sha384(_) => "SHA2-384".to_string(),
            HashAlgorithm::Sha512(_) => "SHA2-512".to_string(),
            HashAlgorithm::Sha3_256(_) => "SHA3-256".to_string(),
            HashAlgorithm::Sha3_384(_) => "SHA3-384".to_string(),
            HashAlgorithm::Sha3_512(_) => "SHA3-512".to_string(),
            HashAlgorithm::Blake3(_) => "BLAKE3".to_string(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct FileChunk {
    pub data: Vec<u8>,
    pub is_last: bool,
}
