# Run a release build (default)
release:
    cargo r --release

# Run a release build with some tracing
dev:
    RUST_BACKTRACE=1 RUST_LOG="debug" cargo r --release

# Run a debug build (very slow)
debug:
    RUST_BACKTRACE=1 RUST_LOG="trace" cargo r

# Run every test
test:
    cargo t --workspace

# Run a headless server and n clients
multi n:
    # Server
    alacritty -e cargo r --release -- --network-mode server &

    # Clients
    for run in $(seq {{n}}); do \
    alacritty -e cargo r --release -- --network-mode client & \
    done

