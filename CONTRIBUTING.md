# Contributing to Runline

Thank you for your interest in contributing to **runline**! Runline is built to be a fast, hyper-minimal, keyboard-first terminal client for watching and acting on GitHub Actions runs.

## Development Setup

### Prerequisites
- **Rust toolchain** (1.75+ or stable) via [rustup](https://rustup.rs/)
- **GitHub CLI** (`gh` 2.40.0 or newer) authenticated via `gh auth login`

### Building & Testing
Clone the repository and run the test suite:

```bash
git clone https://github.com/owner/runline.git
cd runline

# Run unit and integration tests
cargo test

# Run against bundled demo fixtures without making real network requests
cargo run -- --demo
```

### Code Style & Guidelines
- **Formatting:** Keep code formatted with `cargo fmt`.
- **Linting:** Ensure `cargo clippy --all-targets` passes without warnings.
- **Lowercase Copy Rule:** All static copy strings shown in the UI adhere strictly to the lowercase guideline (validated via `./scripts/check_lowercase.sh`).
- **Performance:** Keep rendering snappy with minimal CPU cycles and zero frame diff flickering.

## Pull Requests

1. Fork the repo and create your branch from `main` or `master`.
2. Ensure all tests and clippy checks pass.
3. Open a Pull Request with a clear summary of your changes.
