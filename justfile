# Print the list of commands
help:
    @just --list --unsorted

# Format all code
fmt:
    cargo fmt --all

# Build the monitor
build:
    RUSTFLAGS="-C link-arg=-Tlinker-script.x" cargo +nightly build --target aarch64-unknown-none-softfloat -Zbuild-std=core -Zbuild-std-features=compiler-builtins-mem
    rust-objcopy -O binary ./target/aarch64-unknown-none-softfloat/debug/l4sm artifacts/bl31.bin

# Run the test suite
test:
    cargo test -p capability

# Run the monitor on QEMU
run:
    @just build
    cd artifacts && qemu-system-aarch64 \
      -machine virt,gic-version=3,secure=on,virtualization=on \
      -cpu max \
      -m 1204M \
      -chardev stdio,signal=off,mux=on,id=char0 \
      -monitor chardev:char0 \
      -serial chardev:char0 -serial chardev:char0 \
      -semihosting-config enable=on,target=native \
      -gdb tcp:localhost:1234 \
      -display none \
      -bios bl1.bin

# Start QEMU but wait for GDB to connect
debug:
    @just build
    cd artifacts && qemu-system-aarch64 \
      -machine virt,gic-version=3,secure=on,virtualization=on \
      -cpu max \
      -m 1204M \
      -chardev stdio,signal=off,mux=on,id=char0 \
      -monitor chardev:char0 \
      -serial chardev:char0 -serial chardev:char0 \
      -semihosting-config enable=on,target=native \
      -gdb tcp:localhost:1234 \
      -display none \
      -bios bl1.bin \
      -S

# Start a GDB session
gdb:
    gdb -x ./setup.gdb

# Download the binary artifacts
setup:
    mkdir -p artifacts
    wget -O ./artifacts/bl1.bin https://github.com/CharlyCst/artifact-tfa/releases/download/v0.1.0/bl1_qemu.bin
    wget -O ./artifacts/bl2.bin https://github.com/CharlyCst/artifact-tfa/releases/download/v0.1.0/bl2_qemu.bin
    wget -O ./artifacts/bl32.bin https://github.com/CharlyCst/artifact-rfa/releases/download/v0.1.3/bl32_qemu.bin
    wget -O ./artifacts/bl33.bin https://github.com/CharlyCst/artifact-rfa/releases/download/v0.1.3/bl33_qemu.bin
