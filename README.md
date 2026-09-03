# lumenflow-smart-contract

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/Rust-1.74+-orange)](https://www.rust-lang.org/)
[![Soroban SDK](https://img.shields.io/badge/Soroban_SDK-21.0.0-8B5CF6)](https://soroban.stellar.org/)
[![Tests](https://img.shields.io/badge/tests-10%20passing-brightgreen)](./contracts/stream/src/lib.rs)

> Soroban smart contract powering LumenFlow — real-time continuous streaming payments, milestone vesting, and top-ups on Stellar for open-source maintainers & contributors.

---

## Overview

The stream contract is the on-chain core of LumenFlow. A maintainer locks tokens and specifies a flow rate (tokens per second) to reward contributors working on open-source issues, milestones, and bounties.
- **Cliff Vesting**: Maintainers can set a milestone cliff before which tokens cannot be withdrawn.
- **Top-Ups**: Maintainers can extend existing contributor streams with additional funds at any time.
- **Recipient Address Transfer**: Contributors can safely migrate or rotate their payout wallet.
- **Storage TTL Extension**: Auto-renews Soroban state TTL so long-running streams never expire.

---

## Contract structure

```
contracts/stream/src/
├── lib.rs        — public contract functions + 10 unit tests
├── types.rs      — Stream struct (with title, cliff_time, token), StreamStatus enum
├── storage.rs    — auto-incrementing stream ID, TTL management & read/write helpers
├── events.rs     — on-chain events (CREATED, WITHDRAW, TOP_UP, TRANSFER, CANCEL, PAUSED, RESUMED, COMPLETE)
└── errors.rs     — typed contract error codes
```

---

## Contract functions

| Function | Caller | Description |
|---|---|---|
| `create_stream(sender, recipient, token, deposit, rate_per_second, duration, cliff_time, title)` | Maintainer | Lock tokens and start a stream with issue memo and optional cliff |
| `withdraw(stream_id, recipient)` | Contributor | Pull all accrued tokens unlocked past cliff |
| `deposit_more(stream_id, sender, amount)` | Maintainer | Add more tokens to runway and extend stop time |
| `transfer_recipient(stream_id, current_recipient, new_recipient)` | Contributor | Rotate recipient payout wallet address |
| `cancel_stream(stream_id, sender)` | Maintainer | Accrued → contributor, remainder → maintainer refund |
| `pause_stream(stream_id, sender)` | Maintainer | Freeze accrual |
| `resume_stream(stream_id, sender)` | Maintainer | Resume accrual, shifting time window forward |
| `balance_of(stream_id)` | Anyone | Tokens currently unlocked and available to withdraw |
| `vested_of(stream_id)` | Anyone | Total accrued tokens to date regardless of cliff |
| `get_stream(stream_id)` | Anyone | Full stream state |

---

## Run tests

```bash
cargo test
```

Expected output:
```
running 10 tests
test test::test_create_and_withdraw             ... ok
test test::test_cliff_locks_and_unlocks         ... ok
test test::test_deposit_more_extends_stream     ... ok
test test::test_transfer_recipient             ... ok
test test::test_cancel_splits_funds             ... ok
test test::test_pause_and_resume                ... ok
test test::test_invalid_cliff_rejected          ... ok
test test::test_full_stream_completes           ... ok
test test::test_recipient_cannot_be_sender      ... ok
test test::test_wrong_recipient_cannot_withdraw ... ok

test result: ok. 10 passed; 0 failed
```

## Build WASM

```bash
cargo build --target wasm32-unknown-unknown --release
```

Output: `target/wasm32-unknown-unknown/release/lumenflow_stream.wasm`
