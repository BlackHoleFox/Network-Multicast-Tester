# Network Multicast Tester

This is a small tool to help test a network topography's multicast support. Its extra helpful for testing to see if a access point has properly implemented cross-transport multicasting.

## How to use
Launch the binary and follow the in-terminal steps :)

## Launch Flags

- Broadcaster Mode: --sender
- Receiver Mode: --receiver

## How to get it
1. Download the pre-compiled binary from the [here](https://github.com/BlackHoleFox/Network-Multicast-Tester/releases)
2. Compile from source

## How to build from source
1. Install the latest version of Rust and get the stable toolchain
2. Clone this repository
3. `cd` into the directory you cloned into
4. Run `cargo build` for a debug build or `cargo build --release` for a more optimized binary.
