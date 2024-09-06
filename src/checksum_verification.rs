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
    let (entries, detected_algorithms) = parse_checksum_file(check_file, algorithms)?;
    let algorithms = if !algorithms.is_empty() {
        algorithms
    } else {
        &detected_algorithms
    };

    if show_headers {
        let header = format!(
            "Result  {}  Path",
            algorithms
                .iter()
                .map(|algo| algo.to_string())
                .collect::<Vec<_>>()
                .join("  ")
        );
        output_manager.write_result(&header)?;
    }

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

fn parse_checksum_file(
    path: &Path,
    algorithms: &[HashAlgorithm],
) -> Result<(Vec<ChecksumEntry>, Vec<HashAlgorithm>)> {
    let file = File::open(path).context("Failed to open checksum file")?;
    let reader = BufReader::new(file);
    let mut entries = Vec::new();
    let mut lines = reader.lines();
    let mut detected_algorithms = Vec::new();

    // Check for header
    if let Some(Ok(first_line)) = lines.next() {
        if let Some(header_algorithms) = parse_header(&first_line) {
            detected_algorithms = header_algorithms;
        } else {
            // If it's not a header, parse it as a regular line
            parse_line(
                &first_line,
                algorithms,
                &detected_algorithms,
                &mut entries,
                1,
            )?;
        }
    }

    let algorithms_to_use = if !algorithms.is_empty() {
        algorithms
    } else {
        &detected_algorithms
    };

    for (i, line) in lines.enumerate() {
        let line = line.context(format!("Failed to read line {} from checksum file", i + 2))?;
        parse_line(
            &line,
            algorithms_to_use,
            &detected_algorithms,
            &mut entries,
            i + 2,
        )?;
    }

    Ok((entries, detected_algorithms))
}

fn parse_header(line: &str) -> Option<Vec<HashAlgorithm>> {
    let parts: Vec<&str> = line.split("  ").collect();
    if parts.last() == Some(&"path") {
        let algorithms: Result<Vec<HashAlgorithm>, _> = parts[..parts.len() - 1]
            .iter()
            .map(|&s| HashAlgorithm::new(s))
            .collect();
        algorithms.ok()
    } else {
        None
    }
}

fn parse_line(
    line: &str,
    algorithms: &[HashAlgorithm],
    detected_algorithms: &[HashAlgorithm],
    entries: &mut Vec<ChecksumEntry>,
    line_number: usize,
) -> Result<()> {
    let num_fields = if !algorithms.is_empty() {
        algorithms.len()
    } else if !detected_algorithms.is_empty() {
        detected_algorithms.len()
    } else {
        return Err(anyhow!("No algorithms specified or detected"));
    };

    let parts: Vec<&str> = line.splitn(num_fields + 1, "  ").collect();
    if parts.len() != num_fields + 1 {
        return Err(anyhow!(
            "Invalid checksum file format at line {}",
            line_number
        ));
    }

    entries.push(ChecksumEntry {
        hashes: parts[..num_fields].iter().map(|&s| s.to_string()).collect(),
        path: PathBuf::from(parts[num_fields].trim_end_matches(['\r', '\n'])),
    });

    Ok(())
}

#[derive(Debug)]
struct ChecksumEntry {
    hashes: Vec<String>,
    path: PathBuf,
}
