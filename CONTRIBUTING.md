# Contributing to tld

First off, thank you for considering contributing to tld!

## Getting Started

If you've noticed a bug or have a feature request, check our [Issues](https://github.com/mertcikla/tld-cli/issues) first. If not, go ahead and create one.

## Development

Make sure you have **Rust 1.84** or later installed.
Clone the repository and run the tests:

```bash
git clone https://github.com/Mertcikla/tld.git
cd tld
make test
```

### Speeding Up Development Builds

To speed up Rust builds, we recommend:
1. **[sccache](https://github.com/mozilla/sccache)**: A compiler cache for Rust.
   ```bash
   cargo install sccache
   # Add RUSTC_WRAPPER=sccache to your shell profile
   ```
2. **[mold](https://github.com/rui314/mold)** (Linux) or **[zld](https://github.com/michaeleisel/zld)** (macOS): Faster linkers.
3. **`split-debuginfo = "unpacked"`**: Already configured in `Cargo.toml` for faster macOS linking.

We recommend running the linters before committing:

```bash
make lint
```

## Pull Request Process

1. Fork the repo and create your branch from `main`.
2. Ensure your code passes all tests and linters.
3. Update the documentation if your change requires it.
4. Submit a Pull Request.
