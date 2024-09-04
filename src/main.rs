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
#[command(
    author,
    version,
    about = "ParallelHash: Efficiently calculate cryptographic hashes of files using multiple algorithms in parallel",
    long_about = "ParallelHash is a command-line application that calculates cryptographic hashes of files using multiple algorithms in parallel. It can process individual files or entire directories, and supports MD5, SHA1, SHA256, and SHA512 algorithms. The application is designed to optimize both I/O operations and CPU utilization, making it efficient for various file sizes and storage types."
)]
struct Args {
    #[arg(
        short,
        long,
        value_delimiter = ',',
        help = "Comma-separated list of hash algorithms to use (md5, sha1, sha256, sha512)",
        long_help = "Specify a comma-separated list of hash algorithms to use. Supported algorithms are md5, sha1, sha256, and sha512. Example: -a md5,sha256"
    )]
    algorithms: Vec<String>,

    #[arg(
        short,
        long,
        help = "Verify checksums from the specified file instead of computing new hashes",
        long_help = "Verify checksums from the specified file instead of computing new hashes. The file should contain checksums in the same format as the output of this program."
    )]
    check: Option<PathBuf>,

    #[arg(
        required_unless_present = "check",
        help = "File or directory paths to process",
        long_help = "Specify one or more file or directory paths to process. If a directory is specified, all files within it (including subdirectories) will be processed."
    )]
    paths: Vec<PathBuf>,

    #[arg(
        short,
        long,
        default_value_t = false,
        help = "Show column headers in the output"
    )]
    show_headers: bool,

    #[arg(
        long,
        default_value_t = false,
        help = "Continue processing files even if an error occurs",
        long_help = "Continue processing remaining files even if an error occurs while processing a file. By default, the program stops on the first error."
    )]
    continue_on_error: bool,

    #[arg(
        long,
        default_value_t = false,
        help = "Do not follow symbolic links when processing directories",
        long_help = "Do not follow symbolic links when processing directories. By default, symbolic links are followed."
    )]
    no_follow_symlinks: bool,

    #[arg(
        long,
        default_value_t = 10,
        help = "Size of the channel queue for parallel processing",
        long_help = "Set the size of the channel queue used for parallel processing. A larger value may improve performance but will use more memory. Default is 10."
    )]
    channel_size: usize,

    #[arg(
        long,
        default_value_t = 1024 * 1024,
        help = "Size of each chunk in bytes for file processing (default: 1MB)",
        long_help = "Set the size of each chunk in bytes for file processing. Larger chunks may improve performance but will use more memory. Default is 1MB (1048576 bytes)."
    )]
    chunk_size: usize,
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
        verify_checksums(
            &check_file,
            &algorithms,
            args.show_headers,
            args.channel_size,
            args.chunk_size,
        )?;
    } else {
        compute_hashes(
            &args.paths,
            &algorithms,
            args.show_headers,
            args.continue_on_error,
            !args.no_follow_symlinks,
            args.channel_size,
            args.chunk_size,
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
    channel_size: usize,
    chunk_size: usize,
) -> Result<()> {
    if show_headers {
        println!("{}\t{}", algorithms.len(), "path");
    }

    for path in paths {
        if let Err(e) = process_path(
            path,
            algorithms,
            continue_on_error,
            follow_symlinks,
            channel_size,
            chunk_size,
        ) {
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
    channel_size: usize,
    chunk_size: usize,
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
        match compute_file_hashes(&entry.path, algorithms, channel_size, chunk_size) {
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
    channel_size: usize,
    chunk_size: usize,
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
                        if let Err(e) = process_file(path, algorithms, channel_size, chunk_size) {
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
        process_file(path, algorithms, channel_size, chunk_size)
    }
}

fn process_file(
    path: &Path,
    algorithms: &[HashAlgorithm],
    channel_size: usize,
    chunk_size: usize,
) -> Result<()> {
    match compute_file_hashes(path, algorithms, channel_size, chunk_size) {
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
    channel_size: usize,
    chunk_size: usize,
) -> Result<Vec<String>, HashError> {
    let file = File::open(path).map_err(|e| {
        if e.kind() == io::ErrorKind::NotFound {
            HashError::FileNotFound(e)
        } else {
            HashError::Other(e.into())
        }
    })?;

    let mut reader = BufReader::with_capacity(chunk_size * 2, file);
    let mut buffer = vec![0; chunk_size];

    let (senders, receivers): (Vec<Sender<FileChunk>>, Vec<Receiver<FileChunk>>) =
        algorithms.iter().map(|_| bounded(channel_size)).unzip();

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
        let is_last = bytes_read < chunk_size;
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
