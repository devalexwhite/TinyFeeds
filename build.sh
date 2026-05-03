# Build a smol boy binary

cargo build --release
upx --best --lzma target/release/tinyfeeds
