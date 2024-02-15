echo "    Compiling server..."
(cd server && cargo build --release)
echo "    Running client..."
cd client && cargo run --release
