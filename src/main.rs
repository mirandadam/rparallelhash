use anyhow::{Context, Result};
use clap::Parser;
use crossbeam::channel::{bounded, Receiver};
use digest::Digest;
use md5::Md5;
use sha1::Sha1;
use sha2::{Sha256, Sha512};
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use walkdir::WalkDir;

const CHUNK_SIZE: usize = 1024 * 1024; // 1 MB chunks
const CHANNEL_SIZE: usize = 10; // 10 chunks per channel

#[derive(Clone, Debug)]
enum HashAlgorithm {
    Md5(Md5),
    Sha1(Sha1),
    Sha256(Sha256),
    Sha512(Sha512),
}

impl HashAlgorithm {
    fn new(algo: &str) -> Result<Self> {
        match algo {
            "md5" => Ok(HashAlgorithm::Md5(Md5::new())),
            "sha1" => Ok(HashAlgorithm::Sha1(Sha1::new())),
            "sha256" => Ok(HashAlgorithm::Sha256(Sha256::new())),
            "sha512" => Ok(HashAlgorithm::Sha512(Sha512::new())),
            _ => Err(anyhow::anyhow!("Unsupported algorithm: {}", algo)),
        }
    }

    fn update(&mut self, data: &[u8]) {
        match self {
            HashAlgorithm::Md5(h) => h.update(data),
            HashAlgorithm::Sha1(h) => h.update(data),
            HashAlgorithm::Sha256(h) => h.update(data),
            HashAlgorithm::Sha512(h) => h.update(data),
        }
    }

    fn finalize_reset(&mut self) -> Vec<u8> {
        match self {
            HashAlgorithm::Md5(h) => h.finalize_reset().to_vec(),
            HashAlgorithm::Sha1(h) => h.finalize_reset().to_vec(),
            HashAlgorithm::Sha256(h) => h.finalize_reset().to_vec(),
            HashAlgorithm::Sha512(h) => h.finalize_reset().to_vec(),
        }
    }
}

#[derive(Clone, Debug)]
struct FileChunk {
    data: Vec<u8>,
    is_last: bool,
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, value_delimiter = ',')]
    algorithms: Vec<String>,

    #[arg(required = true)]
    paths: Vec<PathBuf>,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let algorithms = validate_algorithms(&args.algorithms)?;

    // Print header
    println!("{}\t{}", args.algorithms.join("\t"), "path");

    for path in &args.paths {
        if let Err(e) = process_path(path, &algorithms) {
            eprintln!("Error processing path {}: {}", path.display(), e);
        }
    }

    Ok(())
}

fn process_path(path: &Path, algorithms: &[HashAlgorithm]) -> Result<()> {
    if path.is_dir() {
        for entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_file() {
                if let Err(e) = process_file(path, algorithms) {
                    eprintln!("Error processing file {}: {}", path.display(), e);
                }
            }
        }
        Ok(())
    } else {
        process_file(path, algorithms)
    }
}

fn process_file(path: &Path, algorithms: &[HashAlgorithm]) -> Result<()> {
    let file = File::open(path).context(format!("Failed to open file: {}", path.display()))?;
    let mut reader = BufReader::with_capacity(CHUNK_SIZE * 2, file);
    let mut buffer = vec![0; CHUNK_SIZE];

    let (senders, receivers): (Vec<_>, Vec<_>) =
        algorithms.iter().map(|_| bounded(CHANNEL_SIZE)).unzip();

    let results = Arc::new(Mutex::new(Vec::new()));

    // Spawn hash worker threads
    let handles: Vec<_> = algorithms
        .iter()
        .zip(receivers)
        .enumerate()
        .map(|(i, (algo, receiver))| {
            let algo = algo.clone();
            let results = Arc::clone(&results);
            thread::spawn(move || hash_worker(i, algo, receiver, results))
        })
        .collect();

    // Read and distribute chunks
    loop {
        let bytes_read = reader.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        let is_last = bytes_read < CHUNK_SIZE;
        let chunk = FileChunk {
            data: buffer[..bytes_read].to_vec(),
            is_last,
        };

        for sender in &senders {
            sender.send(chunk.clone()).context("Failed to send chunk")?;
        }

        if is_last {
            break;
        }
    }

    // Wait for all workers to finish
    for handle in handles {
        handle.join().expect("Hash worker thread panicked")?;
    }

    // Print results
    let results = results.lock().unwrap();
    let hashes = results
        .iter()
        .map(|r| hex::encode(r))
        .collect::<Vec<_>>()
        .join("\t");
    println!("{}\t{}", hashes, path.display());

    Ok(())
}

fn hash_worker(
    index: usize,
    mut algo: HashAlgorithm,
    receiver: Receiver<FileChunk>,
    results: Arc<Mutex<Vec<Vec<u8>>>>,
) -> Result<()> {
    while let Ok(chunk) = receiver.recv() {
        algo.update(&chunk.data);
        if chunk.is_last {
            let hash = algo.finalize_reset();
            let mut results = results.lock().unwrap();
            if results.len() <= index {
                results.resize(index + 1, Vec::new());
            }
            results[index] = hash;
            break;
        }
    }
    Ok(())
}

fn validate_algorithms(algorithms: &[String]) -> Result<Vec<HashAlgorithm>> {
    algorithms
        .iter()
        .map(|algo| HashAlgorithm::new(algo))
        .collect()
}
