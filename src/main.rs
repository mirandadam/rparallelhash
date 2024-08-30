use anyhow::{Context, Result};
use clap::Parser;
use crossbeam::channel::{bounded, Receiver, Sender};
use digest::Digest;
use md5::Md5;
use rayon::prelude::*;
use sha1::Sha1;
use sha2::{Sha256, Sha512};
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use walkdir::WalkDir;

const CHUNK_SIZE: usize = 1024 * 1024; // 1 MB chunks
const QUEUE_SIZE: usize = 100;

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
    path: Arc<PathBuf>,
    data: Vec<u8>,
    is_last: bool,
}

struct Producer {
    paths: Vec<PathBuf>,
    sender: Sender<FileChunk>,
}

impl Producer {
    fn new(paths: Vec<PathBuf>, sender: Sender<FileChunk>) -> Self {
        Self { paths, sender }
    }

    fn run(&self) -> Result<()> {
        for path in &self.paths {
            self.process_path(path)?;
        }
        Ok(())
    }

    fn process_path(&self, path: &Path) -> Result<()> {
        if path.is_dir() {
            for entry in WalkDir::new(path).into_iter().filter_map(Result::ok) {
                if entry.file_type().is_file() {
                    self.read_file(entry.path())?;
                }
            }
        } else {
            self.read_file(path)?;
        }
        Ok(())
    }

    fn read_file(&self, path: &Path) -> Result<()> {
        let mut file = BufReader::new(File::open(path)?);
        let path = Arc::new(path.to_owned());
        let mut buffer = vec![0; CHUNK_SIZE];

        loop {
            let bytes_read = file.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }
            let is_last = bytes_read < CHUNK_SIZE;
            let chunk = FileChunk {
                path: Arc::clone(&path),
                data: buffer[..bytes_read].to_vec(),
                is_last,
            };
            self.sender.send(chunk).context("Failed to send chunk")?;
            if is_last {
                break;
            }
        }
        Ok(())
    }
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

    let (sender, receiver) = bounded(QUEUE_SIZE);
    let producer = Producer::new(args.paths, sender);

    // Spawn producer thread
    let producer_handle = std::thread::spawn(move || producer.run());

    // Run consumers
    let consumer_results: Vec<Result<()>> = (0..rayon::current_num_threads())
        .into_par_iter()
        .map(|_| consumer(&receiver, &algorithms))
        .collect();

    // Check for errors in producer and consumers
    producer_handle.join().expect("Producer thread panicked")?;
    for result in consumer_results {
        result?;
    }

    Ok(())
}

fn consumer(receiver: &Receiver<FileChunk>, algorithms: &[HashAlgorithm]) -> Result<()> {
    let mut hashers = algorithms.to_vec();
    let mut current_path: Option<Arc<PathBuf>> = None;

    while let Ok(chunk) = receiver.recv() {
        if current_path.as_ref().map(|p| p.as_ref()) != Some(chunk.path.as_ref()) {
            if let Some(path) = current_path.take() {
                let hashes = hashers
                    .iter_mut()
                    .map(|h| hex::encode(h.finalize_reset()))
                    .collect::<Vec<_>>();
                println!("{}\t{}", hashes.join("\t"), path.display());
            }
            current_path = Some(Arc::clone(&chunk.path));
        }

        for hasher in &mut hashers {
            hasher.update(&chunk.data);
        }

        if chunk.is_last {
            let hashes = hashers
                .iter_mut()
                .map(|h| hex::encode(h.finalize_reset()))
                .collect::<Vec<_>>();
            println!("{}\t{}", hashes.join("\t"), chunk.path.display());
            current_path = None;
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
