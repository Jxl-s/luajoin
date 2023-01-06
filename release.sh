RELEASE_VERSION="0.1.1"

# Create the release directory
if [ -d "release" ]; then
    rm -rf release
fi

mkdir release

# Run the build scripts
cargo build --release;
cargo build --release --target x86_64-pc-windows-gnu;
CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER=x86_64-unknown-linux-gnu-gcc \
cargo build --release --target x86_64-unknown-linux-gnu;

# 3. Zip the binaries, in the format luajoin-<platform>-<version>.zip
zip release/luajoin-macos-$RELEASE_VERSION.zip target/release/luajoin
zip release/luajoin-linux-$RELEASE_VERSION.zip target/x86_64-unknown-linux-gnu/release/luajoin
zip release/luajoin-win64-$RELEASE_VERSION.zip target/x86_64-pc-windows-gnu/release/luajoin.exe