# ParallelHash

ParallelHash is a command-line application written in Rust that efficiently calculates cryptographic hashes of files using multiple algorithms in parallel.

## Features

- Supports multiple hash algorithms: MD5, SHA1, SHA256, and SHA512
- Processes files sequentially, but calculates hashes in parallel for each file
- Streams file content, allowing efficient processing of large files without loading them entirely into memory
- Can handle individual files and directories (including subdirectories)
- Optimized for scenarios where file seeks are expensive (e.g., network filesystems)
- Outputs results in a tabular format

## Usage

```bash
parallelhash -a <algorithms> <paths>...
```

- `-a, --algorithms`: Comma-separated list of hash algorithms to use (md5, sha1, sha256, sha512)
- `<paths>`: One or more file or directory paths to process

Example:

```bash
$ parallelhash -a md5,sha256 file1.txt folder/
md5     sha256  path
d41d8cd98f00b204e9800998ecf8427e    e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855    file1.txt
b1946ac92492d2347c6235b4d2611184    5891b5b522d5df086d0ff0b110fbd9d21bb4fc7163af34d08286a2e846f6be03    folder/file2.txt
```

## Design Considerations

ParallelHash is designed to efficiently handle a large number of files, including very large files, in scenarios where file seeks are expensive. The application achieves this by:

1. Processing files sequentially to minimize the impact of expensive file seeks.
2. Streaming file content in chunks, allowing the processing of files larger than available memory.
3. Calculating hashes for multiple algorithms in parallel for each file, maximizing CPU utilization.

This design is particularly beneficial in IO-bound situations, such as when working with network filesystems or slow storage devices. By reading each file only once and performing all hash calculations simultaneously, ParallelHash minimizes IO operations and maximizes throughput.

## Building

To build the project, make sure you have Rust installed, then run:

```bash
cargo build --release
```

The compiled binary will be available in the `target/release` directory.

## License

This project is open-source and available under the MIT License.
