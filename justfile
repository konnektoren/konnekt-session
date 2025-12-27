# Quick commands for running frontend and backend

# Default recipe - shows available commands
default:
    @just --list

# Run frontend with local WebSocket
dev:
    WEBSOCKET_URL=ws://localhost:3000 trunk serve

# Run frontend with production API
prod:
    WEBSOCKET_URL=wss://api.konnektoren.app/session trunk serve

match:
    WEBSOCKET_URL=wss://match.konnektoren.app trunk serve

# Run frontend with Helsing Studio WebSocket
helsing:
    WEBSOCKET_URL=wss://match-0-7.helsing.studio trunk serve

# Run frontend with custom WebSocket URL
custom URL:
    WEBSOCKET_URL={{ URL }} trunk serve

# Run backend server with debug logging
server:
    RUST_LOG=debug cargo run --features=server --bin server

# Run backend server with info logging
server-info:
    RUST_LOG=info cargo run --features=server --bin server

# Run clippy linter
lint:
    cargo clippy --all-features -- -D warnings

# Watch and rebuild on changes (frontend)
watch:
    WEBSOCKET_URL=ws://localhost:3000 trunk serve --watch

# Install required tools
install-tools:
    cargo install trunk
    rustup target add wasm32-unknown-unknown

# Show current WebSocket configuration
show-config:
    @echo "Available WebSocket endpoints:"
    @echo "  Local:      ws://localhost:3000/session"
    @echo "  Production: wss://api.konnektoren.help/session"
    @echo "  Helsing:    wss://match-0-7.helsing.studio"
