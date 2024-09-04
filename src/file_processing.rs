use anyhow::{anyhow, Context, Result};
use crossbeam::channel::{bounded, Receiver, RecvError, Sender};
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use walkdir::WalkDir;

use crate::hash_algorithms::{FileChunk, HashAlgorithm};
use crate::utils::HashError;

pub fn compute_hashes(
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

pub fn compute_file_hashes(
    path: &Path,
    algorithms: &[HashAlgorithm],
    channel_size: usize,
    chunk_size: usize,
) -> Result<Vec<String>, HashError> {
    let file = File::open(path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
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
