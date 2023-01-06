# 1. Run the build scripts
cargo build --release;
cargo build --release --target x86_64-pc-windows-gnu;
CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER=x86_64-unknown-linux-gnu-gcc \
cargo build --release --target x86_64-unknown-linux-gnu;

# 2. Create the release directory
if [ -d "release" ]; then
    rm -rf release
fi

mkdir release

# 3. Move the binaries to the release directory
mv target/release/luajoin release/luajoin-aarch64-apple-darwin
mv target/x86_64-pc-windows-gnu/release/luajoin.exe release/luajoin-x86_64-pc-windows-gnu.exe
mv target/x86_64-unknown-linux-gnu/release/luajoin release/luajoin-x86_64-unknown-linux-gnu