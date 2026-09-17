default: start

start:
    cargo run -p transport

auth:
    cargo run -p transport --bin auth

forecast *args:
    cargo run -p forecast -- .anchor/data {{args}}

lint:
    cargo clippy --workspace --all-targets -- -D warnings

test:
    cargo test --workspace
