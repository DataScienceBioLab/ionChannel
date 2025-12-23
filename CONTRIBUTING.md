# Contributing to ionChannel

## Quick Start

```bash
git clone https://github.com/DataScienceBioLab/ionChannel.git
cd ionChannel
make build
make test
```

## Development Commands

```bash
make build      # Build all crates
make test       # Run tests
make clippy     # Run lints
make fmt        # Format code
make ci         # Full CI check
make demo       # Run integration demo
```

## Code Style

- **Rust 1.75+** with modern async patterns
- `#![forbid(unsafe_code)]` — no unsafe
- Documented public APIs
- Tests for all functionality

```rust
// Good: Type-safe, documented, tested
pub async fn create_session(&self, id: SessionId) -> Result<SessionHandle> {
    // ...
}
```

## Testing

```bash
make test           # All tests
cargo test -p ion-core  # Single crate
cargo test -- --nocapture  # With output
```

## Pull Requests

1. Fork and create a branch
2. Run `make ci` before committing
3. Write clear commit messages
4. Submit PR with description

### Commit Format

```
component: short description

Longer explanation if needed.
```

Examples:
- `ion-core: add TouchDown event`
- `ion-portal: implement SelectDevices`

## Upstream Contribution

ionChannel is designed for upstream contribution to COSMIC:

| Repo | Contribution |
|------|-------------|
| `xdg-desktop-portal-cosmic` | ion-portal |
| `cosmic-comp` | ion-compositor |

### Before Submitting to COSMIC

1. **Discuss first** — Join https://chat.pop-os.org/
2. **Match style** — Follow COSMIC code patterns
3. **Use GPL-3.0** — Required for COSMIC repos

### PR Templates

See `docs/upstream-prs/` for ready-to-use templates.

## Questions?

- **Issues**: Bugs and features
- **COSMIC Chat**: https://chat.pop-os.org/

---

*Thank you for contributing!*
