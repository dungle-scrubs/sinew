# Contributing

Thank you for considering contributing to RustyBar!

## Development Setup

1. Clone the repository:
   ```bash
   git clone https://github.com/dungle-scrubs/rustybar.git
   cd rustybar
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

## Adding a New Module

1. Create a new file in `src/modules/`
2. Implement the `Module` trait
3. Register in `src/modules/mod.rs`
4. Add to `create_module_from_config()` factory
5. Document configuration options

## Pull Request Guidelines

- Keep PRs focused on a single change
- Include a clear description of what and why
- Update documentation if needed
- Add tests for new functionality when applicable
