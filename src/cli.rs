use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about = "ParallelHash: Efficiently calculate cryptographic hashes of files using multiple algorithms in parallel",
    long_about = "ParallelHash is a command-line application that calculates cryptographic hashes of files using multiple algorithms in parallel. It can process individual files or entire directories, and supports MD5, SHA1, SHA256, and SHA512 algorithms. The application is designed to optimize both I/O operations and CPU utilization, making it efficient for various file sizes and storage types."
)]
pub struct Args {
    #[arg(
        short,
        long,
        value_delimiter = ',',
        help = "Comma-separated list of hash algorithms to use (md5, sha1, sha256, sha384, sha512, sha3-256, sha3-384, sha3-512, blake3)",
        long_help = "Specify a comma-separated list of hash algorithms to use. Supported algorithms are md5, sha1, sha256 (or sha2-256), sha384 (or sha2-384), sha512 (or sha2-512), sha3-256, sha3-384, sha3-512, and blake3. Example: -a md5,sha256,blake3"
    )]
    pub algorithms: Vec<String>,

    #[arg(
        short,
        long,
        help = "Verify checksums from the specified file instead of computing new hashes",
        long_help = "Verify checksums from the specified file instead of computing new hashes. The file should contain checksums in the same format as the output of this program."
    )]
    pub check: Option<PathBuf>,

    #[arg(
        required_unless_present = "check",
        help = "File or directory paths to process",
        long_help = "Specify one or more file or directory paths to process. If a directory is specified, all files within it (including subdirectories) will be processed."
    )]
    pub paths: Vec<PathBuf>,

    #[arg(
        short,
        long,
        default_value_t = false,
        help = "Show column headers in the output"
    )]
    pub show_headers: bool,

    #[arg(
        long,
        default_value_t = false,
        help = "Continue processing files even if an error occurs",
        long_help = "Continue processing remaining files even if an error occurs while processing a file. By default, the program stops on the first error."
    )]
    pub continue_on_error: bool,

    #[arg(
        long,
        default_value_t = false,
        help = "Do not follow symbolic links when processing directories",
        long_help = "Do not follow symbolic links when processing directories. By default, symbolic links are followed."
    )]
    pub no_follow_symlinks: bool,

    #[arg(
        long,
        default_value_t = 10,
        help = "Size of the channel queue for parallel processing",
        long_help = "Set the size of the channel queue used for parallel processing. A larger value may improve performance but will use more memory. Default is 10."
    )]
    pub channel_size: usize,

    #[arg(
        long,
        default_value_t = 1024 * 1024,
        help = "Size of each chunk in bytes for file processing (default: 1MB)",
        long_help = "Set the size of each chunk in bytes for file processing. Larger chunks may improve performance but will use more memory. Default is 1MB (1048576 bytes)."
    )]
    pub chunk_size: usize,

    #[arg(
        short,
        long,
        help = "Output file path for results",
        long_help = "Specify a file path to write the results. If not provided, results will be written to stdout."
    )]
    pub output: Option<PathBuf>,
}
