# Trictrac Project Guidelines

## Build & Run Commands
- Build: `cargo build`
- Test: `cargo test`
- Test specific: `cargo test -- test_name`
- Lint: `cargo clippy`
- Format: `cargo fmt`
- Run CLI: `RUST_LOG=info cargo run --bin=client_cli`
- Run CLI with bots: `RUST_LOG=info cargo run --bin=client_cli -- --bot dummy,dummy`
- Build Python lib: `maturin build -m store/Cargo.toml --release`

## Code Style
- Use Rust 2021 edition idioms
- Error handling: Use Result<T, Error> pattern with custom Error types
- Naming: snake_case for functions/variables, CamelCase for types
- Imports: Group standard lib, external crates, then internal modules
- Module structure: Prefer small, focused modules with clear responsibilities
- Documentation: Document public APIs with doc comments
- Testing: Write unit tests in same file as implementation
- Python bindings: Use pyo3 for creating Python modules

## Architecture
- Core game logic in `store` crate
- Multiple clients: CLI, TUI, Bevy (graphical)
- Bot interfaces in `bot` crate