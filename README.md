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

# or read from standard input (no file argument)
cargo run -- < input.stella
```

On success, the tool prints `Type OK` to stdout and exits 0. Type errors are printed to stderr with a non-zero exit code.

## Tests
The project includes both unit tests and a full integration suite:

- **Unit tests**: run with
	```bash
	cargo test
	```

- **Integration suite**: located in `tests/stella_test_suite/` (with subfolders like `stage1/well-typed`, `stage1/ill-typed`, etc). Run with:
	```bash
	cargo test --all-targets
	```
	This will run all unit and suite tests. The suite runner expects the test suite to be present in the above location.

- **Etalon comparison** (optional): to compare against the reference implementation (requires Docker), run:
	```bash
	STELLA_COMPARE_ETALON=1 cargo test suite_compare_etalon -- --nocapture
	```
	This will compare your typechecker’s results with the reference Docker image (`fizruk/stella`).
