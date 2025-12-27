# Contributing to ionChannel

## Quick Start

```bash
git clone https://github.com/DataScienceBioLab/ionChannel.git
cd ionChannel
cargo build
cargo test
```

## Development

```bash
make help       # Show all commands
make build      # Build all crates
make test       # Run tests
make clippy     # Lints
make fmt        # Format
make ci         # Full CI check
```

## Code Style

- Rust 1.75+ with async patterns
- `#![forbid(unsafe_code)]` in core crates
- Documented public APIs
- Tests for all functionality

## Commit Format

```
component: short description

Longer explanation if needed.
```

Examples:
- `ion-core: add RemoteDesktopMode enum`
- `ion-compositor: implement ShmCapture`

## Pull Requests

1. Fork and branch
2. Run `make ci`
3. Submit PR with description

## Upstream Contribution

ionChannel targets COSMIC:

| Repo | Contribution |
|------|-------------|
| `xdg-desktop-portal-cosmic` | ion-portal + capture |
| `cosmic-comp` | ion-compositor |

### Before Submitting to COSMIC

1. Discuss first at https://chat.pop-os.org/
2. Match COSMIC code style
3. Use GPL-3.0 (System76 exception applies)

### PR Templates

See `docs/upstream-prs/` for ready-to-use templates.

## Questions?

- Issues: Bugs and features
- COSMIC Chat: https://chat.pop-os.org/

---

*Thank you for contributing!*
