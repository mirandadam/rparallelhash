use anyhow::{anyhow, Context, Result};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use crate::file_processing::compute_file_hashes;
use crate::hash_algorithms::HashAlgorithm;
use crate::utils::HashError;
use crate::OutputManager;

pub fn verify_checksums(
    check_file: &Path,
    algorithms: &[HashAlgorithm],
    show_headers: bool,
    channel_size: usize,
    chunk_size: usize,
    output_manager: &mut OutputManager,
) -> Result<()> {
    let entries = parse_checksum_file(check_file, algorithms)?;

    if show_headers {
        let header = format!(
            "Result  {}  Path",
            algorithms
                .iter()
                .map(|_| "Hash")
                .collect::<Vec<_>>()
                .join("  ")
        );
        output_manager.write_result(&header)?;
    }

    output_manager.set_total_files(entries.len());

    for entry in entries {
        match compute_file_hashes(
            &entry.path,
            algorithms,
            channel_size,
            chunk_size,
            output_manager,
        ) {
            Ok(computed_hashes) => {
                let result = entry
                    .hashes
                    .iter()
                    .zip(computed_hashes.iter())
                    .all(|(a, b)| a == b);
                let status = if result { "OK" } else { "FAILED" };
                let output = format!(
                    "{}  {}  {}",
                    status,
                    computed_hashes.join("  "),
                    entry.path.display()
                );
                output_manager.write_result(&output)?;
            }
            Err(HashError::FileNotFound(_)) => {
                let output = format!(
                    "FAILED  {}  {}",
                    vec!["N/A"; algorithms.len()].join("  "),
                    entry.path.display()
                );
                output_manager.write_result(&output)?;
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

#[derive(Debug)]
struct ChecksumEntry {
    hashes: Vec<String>,
    path: PathBuf,
}
