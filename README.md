# ⛓️ Birkinlabs Core

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/Rust-1.74.0-orange)](https://www.rust-lang.org/)
[![Soroban SDK](https://img.shields.io/badge/Soroban_SDK-20.0.0-blue)](https://soroban.stellar.org/)

> **Soroban smart contracts powering Birkinlabs on-chain payments and marketplace logic.**

Birkinlabs Core is the on-chain layer of the Birkinlabs Protocol — Rust smart contracts deployed on Stellar/Soroban that handle payment escrow, marketplace listings, seller verification, and on-chain governance for the e-commerce platform.

---

## ✨ Core Features

- 💳 **Payment Contracts**: Trustless token transfers from buyer to seller, with escrow support.
- 🏪 **Marketplace Listings**: On-chain product registry with ownership verification.
- 🔒 **Escrow Vault**: Hold payments until delivery is confirmed — no chargebacks.
- 🏛️ **Governance**: Protocol fee and parameter changes governed by token holders.
- 🪙 **BRK Token**: Native protocol token for fee discounts and governance voting.
- 📦 **Order Settlement**: Automated fund release on delivery confirmation.

---

## 🗂️ Project Structure

```
contracts/
├── src/
│   ├── payment.rs          # Core payment and token transfer logic
│   ├── marketplace.rs      # Product listing registry
│   ├── escrow.rs           # Escrow vault — hold, release, refund
│   ├── governance.rs       # Protocol governance and voting
│   ├── token.rs            # BRK token contract
│   ├── storage.rs          # Persistent contract storage
│   ├── types.rs            # Shared data types
│   ├── events.rs           # Contract event definitions
│   └── errors.rs           # Error codes
scripts/
├── deploy.sh               # Deploy to Stellar network
├── migrate.sh              # Contract migrations
└── setup.sh                # Dev environment setup
tests/
├── payment.test.ts         # Payment contract tests
├── marketplace.test.ts     # Marketplace tests
└── integration.test.ts     # End-to-end tests
```

---

## 🚀 Getting Started

### Prerequisites
- Rust >= 1.74.0
- Soroban CLI
- Funded Stellar testnet account

### Build

```bash
cargo build --target wasm32-unknown-unknown --release
```

### Deploy

```bash
./scripts/setup.sh
./scripts/deploy.sh testnet
```

### Test

```bash
cargo test
```

---

## 📖 Contract Reference

### Payment Flow

1. Buyer connects wallet and adds item to cart.
2. SDK builds a payment transaction to the escrow contract.
3. Buyer signs via Freighter — funds locked on-chain.
4. Seller ships the order.
5. Buyer confirms delivery — escrow releases funds to seller.
6. If disputed, governance jurors vote on resolution.

### BRK Token

The BRK token grants fee discounts on purchases and voting weight in protocol governance. Distributed to early buyers and sellers.

---

## 🗺️ Roadmap

- [ ] **Multi-currency Escrow**: Support USDC, XLM, and custom Stellar assets.
- [ ] **Reputation Oracle**: On-chain seller/buyer reputation scores.
- [ ] **DAO Treasury**: Protocol fees accumulate in community-governed treasury.
- [ ] **Dispute Resolution**: Community juror voting for payment disputes.

---

## 🤝 Community & Support

- **Docs**: [docs.birkinlabs.xyz](https://docs.birkinlabs.xyz)
- **Issues**: [birkinlabs-core/issues](https://github.com/Birkinlabs-Protocol/birkinlabs-core/issues)

---

*Shop freely. Pay trustlessly.*

---

## 📜 License

MIT License. Copyright (c) 2026 Birkinlabs Protocol.
