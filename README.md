# stella-typechecker

Rust implementation of a Stella typechecker (ITMO PL semantics & type systems course). Parsing is generated with `lalrpop` from the Stella grammar, and type checking lives in [src/typechecker.rs](src/typechecker.rs).

## Requirements
- Rust toolchain (stable is fine).
- Optional: Nix manager for a ready-to-use dev shell; any recent Rust install works without Nix.

## Getting started
```bash
# (optional) enter Nix dev shell
nix develop

# build
cargo build

# run on a Stella file
cargo run -- path/to/program.stella
```

On success, the tool prints `Type OK` to stdout and exits 0. Type errors are printed to stderr with a non-zero exit code.

## Tests
```bash
# Full test matrix (unit + Stage 1 suite via integrated test)
cargo test --all-targets

# Or run the suite runner directly (honors $CARGO_TARGET_DIR, builds if missing)
cargo run --bin run_suite --
```
