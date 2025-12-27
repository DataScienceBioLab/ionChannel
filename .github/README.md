# GitHub Configuration

This directory contains GitHub-specific configuration files.

## CI/CD

GitHub Actions workflows should be placed in `.github/workflows/`.

## Recommended Workflow

Create `.github/workflows/ci.yml`:

```yaml
name: CI

on:
  push:
    branches: [ main ]
  pull_request:
    branches: [ main ]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      
      - name: Build
        run: cargo build --workspace --release
      
      - name: Test
        run: cargo test --workspace
      
      - name: Clippy
        run: cargo clippy --workspace --all-targets -- -D warnings
      
      - name: Format
        run: cargo fmt --all -- --check
      
      - name: Benchmarks
        run: cargo bench --no-run

  coverage:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      
      - name: Install tarpaulin
        run: cargo install cargo-tarpaulin
      
      - name: Generate coverage
        run: cargo tarpaulin --workspace --out xml
      
      - name: Upload coverage
        uses: codecov/codecov-action@v3
```

## Status

Current CI status: See badge in main README.md

