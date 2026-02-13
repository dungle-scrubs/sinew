# Contributing

Thank you for considering contributing to Sinew!

## Development Setup

1. Clone the repository:
   ```bash
   git clone https://github.com/dungle-scrubs/sinew.git
   cd sinew
   ```

2. Install Rust (if not already installed):
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

3. Build the project:
   ```bash
   cargo build
   ```

4. Run with debug logging:
   ```bash
   RUST_LOG=debug cargo run
   ```

## Making Changes

1. Fork the repository
2. Create a feature branch: `git checkout -b feature/my-feature`
3. Make your changes
4. Run the linter: `cargo clippy --all-targets`
5. Format code: `cargo fmt`
6. Run tests: `cargo test`
7. Commit with a descriptive message
8. Push and open a Pull Request

## Code Style

- Run `cargo fmt` before committing
- Address all `cargo clippy` warnings
- Follow Rust naming conventions
- Add comments for non-obvious logic

## Commit Messages

This project uses [Conventional Commits](https://www.conventionalcommits.org/) for automated changelog generation:

| Prefix | Purpose | Example |
|--------|---------|---------|
| `feat:` | New feature | `feat: add volume module` |
| `fix:` | Bug fix | `fix: resolve crash on empty config` |
| `docs:` | Documentation | `docs: update README` |
| `refactor:` | Code restructuring | `refactor: extract popup logic` |
| `chore:` | Maintenance | `chore: update dependencies` |

## Adding a New Module

1. Create a new file in `src/gpui_app/modules/`
2. Implement the `GpuiModule` trait
3. Register the factory in `src/gpui_app/modules/mod.rs`
4. Add the type to `KNOWN_MODULE_TYPES` in `src/config/types.rs`
5. Document configuration options

## Pull Request Guidelines

- Keep PRs focused on a single change
- Include a clear description of what and why
- Update documentation if needed
- Add tests for new functionality when applicable
