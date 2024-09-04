use anyhow::Result;
use clap::Parser;

mod checksum_verification;
mod cli;
mod file_processing;
mod hash_algorithms;
mod utils;

use cli::Args;

fn main() -> Result<()> {
    let args = Args::parse();
    let algorithms = utils::validate_algorithms(&args.algorithms)?;

    if let Some(check_file) = args.check {
        checksum_verification::verify_checksums(
            &check_file,
            &algorithms,
            args.show_headers,
            args.channel_size,
            args.chunk_size,
        )?;
    } else {
        file_processing::compute_hashes(
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
