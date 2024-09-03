use anyhow::{anyhow, Context, Result};
use clap::Parser;
use crossbeam::channel::{bounded, Receiver, RecvError, Sender};
use digest::Digest;
use md5::Md5;
use sha1::Sha1;
use sha2::{Sha256, Sha512};
use std::fs::File;
use std::io::{self, BufRead, BufReader, Read};
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
        match algo.to_lowercase().as_str() {
            "md5" => Ok(HashAlgorithm::Md5(Md5::new())),
            "sha1" => Ok(HashAlgorithm::Sha1(Sha1::new())),
            "sha256" => Ok(HashAlgorithm::Sha256(Sha256::new())),
            "sha512" => Ok(HashAlgorithm::Sha512(Sha512::new())),
            _ => Err(anyhow!("Unsupported algorithm: {}", algo)),
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

    #[arg(short, long)]
    check: Option<PathBuf>,

    #[arg(required_unless_present = "check")]
    paths: Vec<PathBuf>,

    #[arg(short, long, default_value_t = false)]
    show_headers: bool,

    #[arg(long, default_value_t = false)]
    continue_on_error: bool,

    #[arg(long, default_value_t = true)]
    follow_symlinks: bool,
}

#[derive(Debug)]
struct ChecksumEntry {
    hashes: Vec<String>,
    path: PathBuf,
}

#[derive(Debug)]
enum HashError {
    FileNotFound(io::Error),
    Other(anyhow::Error),
}

impl From<anyhow::Error> for HashError {
    fn from(err: anyhow::Error) -> Self {
        HashError::Other(err)
    }
}

fn main() -> Result<()> {
    let args = Args::parse();
    let algorithms = validate_algorithms(&args.algorithms)?;

    if let Some(check_file) = args.check {
        verify_checksums(&check_file, &algorithms, args.show_headers)?;
    } else {
        compute_hashes(
            &args.paths,
            &algorithms,
            args.show_headers,
            args.continue_on_error,
            args.follow_symlinks,
        )?;
    }

    Ok(())
}

fn compute_hashes(
    paths: &[PathBuf],
    algorithms: &[HashAlgorithm],
    show_headers: bool,
    continue_on_error: bool,
    follow_symlinks: bool,
) -> Result<()> {
    if show_headers {
        println!("{}\t{}", algorithms.len(), "path");
    }

    for path in paths {
        if let Err(e) = process_path(path, algorithms, continue_on_error, follow_symlinks) {
            eprintln!("Error processing path {}: {}", path.display(), e);
            if !continue_on_error {
                return Err(e);
            }
        }
    }

    Ok(())
}

fn verify_checksums(
    check_file: &Path,
    algorithms: &[HashAlgorithm],
    show_headers: bool,
) -> Result<()> {
    let entries = parse_checksum_file(check_file, algorithms)?;

    if show_headers {
        println!(
            "Result\t{}\tPath",
            algorithms
                .iter()
                .map(|_| "Hash")
                .collect::<Vec<_>>()
                .join("\t")
        );
    }

    for entry in entries {
        match compute_file_hashes(&entry.path, algorithms) {
            Ok(computed_hashes) => {
                let result = entry
                    .hashes
                    .iter()
                    .zip(computed_hashes.iter())
                    .all(|(a, b)| a == b);
                let status = if result { "OK" } else { "FAILED" };
                println!(
                    "{}\t{}\t{}",
                    status,
                    computed_hashes.join("\t"),
                    entry.path.display()
                );
            }
            Err(HashError::FileNotFound(_)) => {
                println!(
                    "FAILED\t{}\t{}",
                    vec!["N/A"; algorithms.len()].join("\t"),
                    entry.path.display()
                );
            }
            Err(HashError::Other(e)) => {
                eprintln!("Error computing hashes for {}: {}", entry.path.display(), e);
            }
        }
    }

    Ok(())
}

fn parse_checksum_file(path: &Path, algorithms: &[HashAlgorithm]) -> Result<Vec<ChecksumEntry>> {
    let file = File::open(path).context("Failed to open checksum file")?;
    let reader = BufReader::new(file);
    let mut entries = Vec::new();

    for (i, line) in reader.lines().enumerate() {
        let line = line.context(format!("Failed to read line {} from checksum file", i + 1))?;
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() != algorithms.len() + 1 {
            return Err(anyhow!("Invalid checksum file format at line {}", i + 1));
        }

        entries.push(ChecksumEntry {
            hashes: parts[..algorithms.len()]
                .iter()
                .map(|&s| s.to_string())
                .collect(),
            path: PathBuf::from(parts[algorithms.len()]),
        });
    }

    Ok(entries)
}

fn process_path(
    path: &Path,
    algorithms: &[HashAlgorithm],
    continue_on_error: bool,
    follow_symlinks: bool,
) -> Result<()> {
    if path.is_symlink() && !follow_symlinks {
        println!(
            "{}\t{} (symlink)",
            vec!["N/A"; algorithms.len()].join("\t"),
            path.display()
        );
        return Ok(());
    }

    if path.is_dir() {
        for entry in WalkDir::new(path).follow_links(follow_symlinks) {
            match entry {
                Ok(entry) => {
                    let path = entry.path();
                    if path.is_file() {
                        if let Err(e) = process_file(path, algorithms) {
                            eprintln!("Error processing file {}: {}", path.display(), e);
                            if !continue_on_error {
                                return Err(anyhow!("Failed to process file: {}", path.display()));
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error accessing entry: {}", e);
                    if !continue_on_error {
                        return Err(anyhow!("Failed to access entry"));
                    }
                }
            }
        }
        Ok(())
    } else {
        process_file(path, algorithms)
    }
}

fn process_file(path: &Path, algorithms: &[HashAlgorithm]) -> Result<()> {
    match compute_file_hashes(path, algorithms) {
        Ok(hashes) => {
            println!("{}\t{}", hashes.join("\t"), path.display());
            Ok(())
        }
        Err(HashError::FileNotFound(e)) => {
            println!(
                "{}\t{} (File not found: {})",
                vec!["N/A"; algorithms.len()].join("\t"),
                path.display(),
                e
            );
            Ok(())
        }
        Err(HashError::Other(e)) => Err(e),
    }
}

fn compute_file_hashes(
    path: &Path,
    algorithms: &[HashAlgorithm],
) -> Result<Vec<String>, HashError> {
    let file = File::open(path).map_err(|e| {
        if e.kind() == io::ErrorKind::NotFound {
            HashError::FileNotFound(e)
        } else {
            HashError::Other(e.into())
        }
    })?;

    let mut reader = BufReader::with_capacity(CHUNK_SIZE * 2, file);
    let mut buffer = vec![0; CHUNK_SIZE];

    let (senders, receivers): (Vec<Sender<FileChunk>>, Vec<Receiver<FileChunk>>) =
        algorithms.iter().map(|_| bounded(CHANNEL_SIZE)).unzip();

    let results = Arc::new(Mutex::new(Vec::new()));

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

    loop {
        let bytes_read = reader
            .read(&mut buffer)
            .with_context(|| format!("Failed to read from file: {}", path.display()))?;
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

    for handle in handles {
        handle
            .join()
            .map_err(|e| anyhow!("Hash worker thread panicked: {:?}", e))??;
    }

    let results = results
        .lock()
        .map_err(|e| anyhow!("Failed to lock results: {:?}", e))?;
    Ok(results.iter().map(|r| hex::encode(r)).collect())
}

fn hash_worker(
    index: usize,
    mut algo: HashAlgorithm,
    receiver: Receiver<FileChunk>,
    results: Arc<Mutex<Vec<Vec<u8>>>>,
) -> Result<()> {
    loop {
        match receiver.recv() {
            Ok(chunk) => {
                algo.update(&chunk.data);
                if chunk.is_last {
                    let hash = algo.finalize_reset();
                    let mut results = results
                        .lock()
                        .map_err(|e| anyhow!("Failed to lock results: {:?}", e))?;
                    if results.len() <= index {
                        results.resize(index + 1, Vec::new());
                    }
                    results[index] = hash;
                    break;
                }
            }
            Err(RecvError) => {
                // Channel is disconnected, exit the loop
                break;
            }
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
