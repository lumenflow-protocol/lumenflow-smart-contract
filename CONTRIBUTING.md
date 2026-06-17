# Contributing to lumenflow-smart-contract

This repo contains the Soroban smart contracts for LumenFlow — the on-chain streaming payment logic written in Rust.

---

## Prerequisites

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Add WASM target
rustup target add wasm32-unknown-unknown

# Install Stellar CLI
cargo install --locked stellar-cli --features opt
```

---

## Development workflow

```bash
# Clone
git clone https://github.com/lumenflow-protocol/lumenflow-smart-contract.git
cd lumenflow-smart-contract

# Run tests — must pass before any PR
cargo test

# Check formatting
cargo fmt --check

# Run linter
cargo clippy -- -D warnings

# Build WASM
cargo build --target wasm32-unknown-unknown --release
```

---

## Where things live

```
contracts/stream/src/
├── lib.rs        — all public contract functions + tests
├── types.rs      — Stream struct, StreamStatus enum
├── storage.rs    — stream ID counter, read/write helpers
├── events.rs     — on-chain events emitted per action
└── errors.rs     — StreamError enum
```

---

## Rules for contract changes

- Every new public function **must** have a test in `lib.rs`
- Use `contracterror` + `Result<T, StreamError>` — no `panic!` or `unwrap` in contract code
- Keep each file focused on its responsibility — don't mix logic across files
- Run `cargo fmt` before committing

---

## Branch naming

```
feat/resume-stream-logic
fix/balance-underflow
test/cancel-with-withdrawal
docs/contract-api-reference
```

---

## Commit format

```
feat: add resume_stream function
fix: clamp balance to deposit ceiling
test: add cancel after partial withdrawal test
```

---

## Opening a PR

- Target branch: `main`
- Include a short description of what changed and why
- `cargo test` output must show all tests passing

---

For questions, open a GitHub Issue or Discussion.
