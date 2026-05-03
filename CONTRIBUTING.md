# Contributing to farol

Thanks for wanting to help. A few practical notes so your first contribution lands smoothly.

## Building from source

Requirements:
- Rust 1.75 or newer (`rustup install stable`)
- Python 3.9+
- [uv](https://docs.astral.sh/uv/) (`curl -LsSf https://astral.sh/uv/install.sh | sh`)

```bash
git clone https://github.com/ferrumio/farol.git
cd farol
uv venv
uv pip install maturin
uv run maturin develop       # builds the Python wheel into the uv-managed venv
cargo build --workspace
```

Run the CLI:

```bash
cargo run -p farol-cli -- --help
```

Run tests:

```bash
cargo test --workspace
```

## Coding style

- Rust: `cargo fmt` and `cargo clippy -- -D warnings` must pass.
- Python: `uv run ruff check` and `uv run ruff format` must pass.
- Keep changes focused; separate refactors from features in their own PRs.
- Write tests for new behavior.

## Commit messages

We use Conventional Commits. Examples:

- `feat(core): parse frontmatter`
- `fix(render): preserve trailing slashes on internal links`
- `chore(ci): bump actions/checkout to v4`

## Pull requests

- Link the issue it closes (`Closes #42`).
- Describe *why*, not just *what*. The diff shows what changed; your PR description should explain the motivation.
- Keep PRs small. Reviewers move faster on focused changes.

## RFCs

Breaking changes to the plugin API, the config schema, or the theme contract go through an RFC:

1. Copy `rfcs/0000-template.md` to `rfcs/NNNN-short-name.md`.
2. Open a PR for discussion.
3. Merge when there is rough consensus. Implementation lives in a separate PR.

## Code of Conduct

This project follows the [Contributor Covenant](CODE_OF_CONDUCT.md). Be kind.

## License

By contributing, you agree that your contributions are licensed under the Apache License 2.0. There is no separate CLA.
