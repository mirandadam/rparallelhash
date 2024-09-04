# Backlog

0. Implement hash verification from a file, like `md5sum -c`. This will be a bit more complex since it is necessary to either receive column headers or manually identify the algorithms being used on the command line. Also it is nececessary to properly handle collisions such as the ones discovered for md5 and sha1.

1. Error Handling: The current implementation uses the `anyhow` crate for error handling, which is a good choice for simplifying error management. However, you might want to consider creating custom error types for more specific error handling in the future, especially if you plan to expand the application's functionality.

2. Testing: The README mentions that the recent changes haven't been tested yet. It would be beneficial to add unit tests and integration tests to ensure the application behaves as expected, especially given its parallel nature.

3. Performance Monitoring: Consider adding optional performance metrics (e.g., time taken, throughput) to help users understand the benefits of the parallel approach in their specific use cases.

4. Documentation: While the README now provides a good overview, it might be helpful to add inline documentation (comments) to the `main.rs` file, especially for the `Producer` and `consumer` functions, to explain their roles in the streaming and parallel processing approach.

5. Benchmarking: Develop a set of benchmarks to measure the performance impact of different channel and chunk sizes across various file types and sizes. This could help provide guidance on optimal settings for different use cases.
