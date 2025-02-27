# Run a release build (default)
release:
    cargo r --release

# Run a release build with a large view distance
far:
    cargo r --release -- --view-distance 256 --load-distance 288

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
    alacritty --hold -e cargo r --release -- --network-mode server --player-name name_server &

    # Clients
    for i in $(seq {{n}}); do \
    alacritty --hold -e cargo r --release -- --network-mode client --player-name name_client_${i} & \
    done

