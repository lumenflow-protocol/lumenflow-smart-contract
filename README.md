# lumenflow-smart-contract

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/Rust-1.74+-orange)](https://www.rust-lang.org/)
[![Soroban SDK](https://img.shields.io/badge/Soroban_SDK-21.0.0-8B5CF6)](https://soroban.stellar.org/)
[![Tests](https://img.shields.io/badge/tests-5%20passing-brightgreen)](./contracts/stream/src/lib.rs)

> Soroban smart contract powering LumenFlow — real-time per-second payment streaming on Stellar.

---

## Overview

The stream contract is the on-chain core of LumenFlow. A sender deposits tokens and specifies a rate (tokens per second). The contract tracks elapsed time and releases the proportional amount to the recipient — who can withdraw at any time. The sender can pause, resume, or cancel the stream at any point.

---

## Contract structure

```
contracts/stream/src/
├── lib.rs        — public contract functions + tests
├── types.rs      — Stream struct, StreamStatus enum
├── storage.rs    — auto-incrementing stream ID, read/write helpers
├── events.rs     — on-chain events (CREATED, WITHDRAW, CANCEL, PAUSED, RESUMED)
└── errors.rs     — error codes
```

---

## Contract functions

| Function | Caller | Description |
|---|---|---|
| `create_stream(sender, recipient, token, deposit, rate_per_second, duration)` | Sender | Lock tokens and start a stream |
| `withdraw(stream_id, recipient)` | Recipient | Pull all accrued tokens |
| `cancel_stream(stream_id, sender)` | Sender | Accrued → recipient, remainder → sender |
| `pause_stream(stream_id, sender)` | Sender | Freeze accrual |
| `resume_stream(stream_id, sender)` | Sender | Resume accrual, shifting time window |
| `balance_of(stream_id)` | Anyone | Tokens available to withdraw right now |
| `get_stream(stream_id)` | Anyone | Full stream state |

### Balance calculation

```
effective_end = min(now, stop_time)
elapsed       = effective_end - start_time + elapsed_before_pause
streamed      = min(elapsed × rate_per_second, deposit)
available     = streamed - withdrawn
```

---

## Getting started

### Prerequisites

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Add WASM target
rustup target add wasm32-unknown-unknown

# Install Stellar CLI
cargo install --locked stellar-cli --features opt
```

### Run tests

```bash
cargo test
```

Expected output:
```
running 5 tests
test test::test_create_and_withdraw     ... ok
test test::test_cancel_splits_funds     ... ok
test test::test_pause_and_resume        ... ok
test test::test_full_stream_completes   ... ok
test test::test_wrong_recipient_cannot_withdraw ... ok

test result: ok. 5 passed; 0 failed
```

### Build WASM

```bash
cargo build --target wasm32-unknown-unknown --release
```

Output: `target/wasm32-unknown-unknown/release/lumenflow_stream.wasm`

### Deploy to testnet

```bash
# Fund an account
stellar keys generate --global alice --network testnet --fund

# Deploy
stellar contract deploy \
  --wasm target/wasm32-unknown-unknown/release/lumenflow_stream.wasm \
  --source alice \
  --network testnet
```

---

## Events emitted

| Event | Topics | Value |
|---|---|---|
| `CREATED` | `[CREATED, stream_id]` | `(sender, recipient, deposit, rate_per_second, start_time, stop_time)` |
| `WITHDRAW` | `[WITHDRAW, stream_id]` | `(recipient, amount)` |
| `CANCEL` | `[CANCEL, stream_id]` | `(sender_refund, recipient_payout)` |
| `PAUSED` | `[PAUSED, stream_id]` | `()` |
| `RESUMED` | `[RESUMED, stream_id]` | `()` |

---

## Contributing

See the root [CONTRIBUTING.md](../CONTRIBUTING.md).

## License

MIT License — Copyright (c) 2026 LumenFlow Protocol.
