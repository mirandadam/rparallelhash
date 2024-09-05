# ParallelHash

ParallelHash is a command-line application written in Rust that efficiently calculates cryptographic hashes of files using multiple algorithms in parallel.

## Features

- Supports multiple hash algorithms: MD5, SHA1, SHA256 (SHA2-256), SHA384 (SHA2-384), SHA512 (SHA2-512), SHA3-256, SHA3-384, SHA3-512, and BLAKE3
- Processes files sequentially, one at a time
- Calculates hashes for different algorithms in parallel for each chunk of data
- Streams file content, allowing efficient processing of large files without loading them entirely into memory
- Can handle individual files and directories (including subdirectories)
- Optimized for both I/O-bound and CPU-bound scenarios
- Outputs results in a tabular format

## Usage

```bash
parallelhash [OPTIONS] [PATHS]...
```

### Arguments

- `[PATHS]...`: Specify one or more file or directory paths to process. If a directory is specified, all files within it (including subdirectories) will be processed.

### Options

- `-a, --algorithms <ALGORITHMS>`: Specify a comma-separated list of hash algorithms to use. Supported algorithms are md5, sha1, sha256 (or sha2-256), sha384 (or sha2-384), sha512 (or sha2-512), sha3-256, sha3-384, sha3-512, and blake3. Example: [`-a md5,sha256,blake3`]
- `-c, --check <CHECK>`: Verify checksums from the specified file instead of computing new hashes. The file should contain checksums in the same format as the output of this program.
- `-s, --show-headers`: Show column headers in the output.
- `--continue-on-error`: Continue processing remaining files even if an error occurs while processing a file. By default, the program stops on the first error.
- `--no-follow-symlinks`: Do not follow symbolic links when processing directories. By default, symbolic links are followed.
- `--channel-size <CHANNEL_SIZE>`: Set the size of the channel queue used for parallel processing. A larger value may improve performance but will use more memory. Default is 10.
- `--chunk-size <CHUNK_SIZE>`: Set the size of each chunk in bytes for file processing. Larger chunks may improve performance but will use more memory. Default is 1MB (1048576 bytes).
- `-o, --output <OUTPUT>`: Specify a file path to write the results. If not provided, results will be written to stdout.
- `-h, --help`: Print help (see a summary with '-h').
- `-V, --version`: Print version.

### Example

```bash
$ parallelhash -a md5,sha256,blake3 --channel-size 20 --chunk-size 2097152 --no-follow-symlinks file1.txt folder/
MD5  SHA2-256  BLAKE3  path
d41d8cd98f00b204e9800998ecf8427e  e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855  af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262  file1.txt
b1946ac92492d2347c6235b4d2611184  5891b5b522d5df086d0ff0b110fbd9d21bb4fc7163af34d08286a2e846f6be03  256c83b297114d201b30179f3f0ef0cace9783622da5974326b436178aeef610  folder/file2.txt
```

Note: SHA256, SHA384, and SHA512 can also be referred to as SHA2-256, SHA2-384, and SHA2-512 respectively.

## Design Considerations

ParallelHash is designed to efficiently handle files of various sizes, optimizing for both I/O-bound and CPU-bound scenarios. The application achieves this by:

1. Processing files sequentially, one at a time, to minimize the impact of expensive file seeks.
2. Streaming file content in chunks using a buffered reader, allowing the processing of files larger than available memory.
3. Calculating hashes for multiple algorithms in parallel for each chunk of data, maximizing CPU utilization.
4. Using separate threads and bounded channels for each hashing algorithm, allowing for efficient parallel processing.

## Building

To build the project, make sure you have Rust installed, then run:

```bash
cargo build --release
```

The compiled binary will be available in the `target/release` directory.

## License

This project is open-source and available under the MIT License.
