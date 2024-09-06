use anyhow::Result;
use clap::Parser;
use parallelhash::{
    checksum_verification, compute_hashes, validate_algorithms, Args, OutputManager,
};

fn main() -> Result<()> {
    let args = Args::parse();
    let algorithms = validate_algorithms(&args.algorithms)?;

    let mut output_manager = OutputManager::new(args.output.as_deref())?;

    if let Some(check_file) = args.check {
        if !args.algorithms.is_empty() {
            eprintln!("Warning: Algorithms specified with -a option will take precedence over the header in the checksum file.");
        }
        checksum_verification::verify_checksums(
            &check_file,
            &algorithms,
            args.show_headers,
            args.channel_size,
            args.chunk_size,
            &mut output_manager,
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
            &mut output_manager,
        )?;
    }

    Ok(())
}
