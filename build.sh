# Build a smol boy binary

cargo build --release
if command -v upx &> /dev/null; then
    upx --best --lzma target/release/TinyFeeds
else
    echo "upx not found, skipping compression."
fi
