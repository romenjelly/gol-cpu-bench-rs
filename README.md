# Using Conway's Game of Life to Benchmark Processors

This is a simple benchmarking tool written in rust that can show an indicative result of multithreaded performance.

It runs for a configurable amount of iterations simulating Conway's Game of Life generations on a fixed-size buffer.
Results are reported in iterations per second.

The program will, by default, run 1024 iterations over a 3840 by 2160 buffer using as many threads as the machine reports.

The `work_slice_len` which can bee seen in the config file indicates how many items each thread processes when going through the buffer.  
A value of 1 would mean that most of CPU cycles are spent in cross-thread IO overhead.  
The `work_slice_len` should be something that ideally fits in your closest CPU cache (considering each item is 1 byte). Default value is 128 * 128.

## Result Validity

The accuracy or validity of the results is highly questionable, since it doesn't do any "useful" computations, it's mostly a matter of index math and simple boolean logic.

Use a more complex program to benchmark your PC, such as a game or a raytracer.

## Building and Running

This project use Rust stable, so a stable rust toolchain needs to be installed.  
See [rust installation guide](https://www.rust-lang.org/tools/install).

The debug version of this tool is extremely slow, so run it using `cargo run --release`.

To add flags, add a `--` after `--release`, example: `cargo run --release -- --help`.

More details about flags can be found using the `--help` flag.

## Configuration File

If you want to configure your bencmark run, launch the program with `--generate-config <filename?>`, which will generate a configuration file.  
The file will by default be named `benc_config.toml`, but you can specify the name yourself (omitting the `.toml` extension).

To have the program use said configuration file, you need to run it with the `--use-config <filename?>` flag.  
The program will by default search for the `bench_config.toml` file, but you can specify your own (again, omitting the `.toml` extension).
