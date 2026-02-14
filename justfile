# Print the list of commands
help:
    @just --list --unsorted

# Format all code
fmt:
    cargo fmt --all

# Build the monitor
build:
    RUSTFLAGS="-C link-arg=-Tlinker-script.x" cargo +nightly build --target aarch64-unknown-none-softfloat -Zbuild-std=core -Zbuild-std-features=compiler-builtins-mem
