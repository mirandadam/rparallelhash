# ParallelHash

ParallelHash is a command-line application written in Rust that efficiently calculates cryptographic hashes of files using multiple algorithms in parallel.

## Features

- Supports multiple hash algorithms: MD5, SHA1, SHA256, and SHA512
- Processes files sequentially, one at a time
- Calculates hashes for different algorithms in parallel for each chunk of data
- Streams file content, allowing efficient processing of large files without loading them entirely into memory
- Can handle individual files and directories (including subdirectories)
- Optimized for both I/O-bound and CPU-bound scenarios
- Outputs results in a tabular format

## Usage

```bash
parallelhash [OPTIONS] -a <algorithms> <paths>...
```

- `-a, --algorithms`: Comma-separated list of hash algorithms to use (md5, sha1, sha256, sha512)
- `<paths>`: One or more file or directory paths to process
- `--channel-size <SIZE>`: Size of the channel queue (default: 10)
- `--chunk-size <SIZE>`: Size of each chunk in bytes (default: 1048576, which is 1 MB)
- `--no-follow-symlinks`: Do not follow symbolic links when processing directories

Example:

```bash
$ parallelhash -a md5,sha256 --channel-size 20 --chunk-size 2097152 --no-follow-symlinks file1.txt folder/
md5     sha256  path
d41d8cd98f00b204e9800998ecf8427e    e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855    file1.txt
b1946ac92492d2347c6235b4d2611184    5891b5b522d5df086d0ff0b110fbd9d21bb4fc7163af34d08286a2e846f6be03    folder/file2.txt
```

## Design Considerations

ParallelHash is designed to efficiently handle files of various sizes, optimizing for both I/O-bound and CPU-bound scenarios. The application achieves this by:

1. Processing files sequentially, one at a time, to minimize the impact of expensive file seeks.
2. Streaming file content in chunks using a buffered reader, allowing the processing of files larger than available memory.
3. Calculating hashes for multiple algorithms in parallel for each chunk of data, maximizing CPU utilization.
4. Using separate threads and bounded channels for each hashing algorithm, allowing for independent processing speeds.
5. Implementing a backpressure mechanism to naturally balance between I/O and CPU operations.
6. Providing configurable channel and chunk sizes to fine-tune performance for specific use cases.

This design is particularly beneficial in various situations, such as when working with network filesystems, slow storage devices, or when processing computationally expensive hash algorithms. By reading each file only once and performing hash calculations in parallel, ParallelHash optimizes both I/O operations and CPU utilization.

The configurable channel and chunk sizes allow users to adjust the balance between memory usage and performance. Larger channel sizes can improve throughput by reducing the likelihood of worker threads waiting for data, while larger chunk sizes can reduce the overhead of channel operations at the cost of increased memory usage.

## Building

To build the project, make sure you have Rust installed, then run:

```bash
cargo build --release
```

The compiled binary will be available in the `target/release` directory.

## License

This project is open-source and available under the MIT License.
