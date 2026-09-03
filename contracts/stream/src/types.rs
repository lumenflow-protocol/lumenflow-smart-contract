use soroban_sdk::{contracttype, Address, String};

#[contracttype]
#[derive(Clone, PartialEq, Debug)]
pub enum StreamStatus {
    Active,
    Paused,
    Cancelled,
    Completed,
}

/// A payment stream — maintainer/sender locks tokens and they drip to contributor/recipient over time.
#[contracttype]
#[derive(Clone)]
pub struct Stream {
    pub id: u64,
    pub sender: Address,
    pub recipient: Address,
    /// The token contract address (XLM, USDC, etc.)
    pub token: Address,
    /// Total tokens deposited into this stream.
    pub deposit: i128,
    /// Tokens released per second.
    pub rate_per_second: i128,
    /// Ledger timestamp when stream starts.
    pub start_time: u64,
    /// Ledger timestamp when stream fully drains.
    pub stop_time: u64,
    /// Milestone cliff timestamp before which recipient cannot withdraw (0 if no cliff).
    pub cliff_time: u64,
    /// Stream title or memo (e.g. "Issue #12: Fix Indexer", "Contributor Monthly Grant").
    pub title: String,
    /// Tokens already withdrawn by the recipient.
    pub withdrawn: i128,
    /// Elapsed seconds already streamed before the last pause (only used when paused).
    pub elapsed_before_pause: u64,
    /// Ledger timestamp when the stream was last paused (0 if not paused).
    pub paused_at: u64,
    pub status: StreamStatus,
}
