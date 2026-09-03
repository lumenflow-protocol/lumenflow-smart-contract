#![no_std]

mod errors;
mod events;
mod storage;
mod types;

use errors::StreamError;
use soroban_sdk::{contract, contractimpl, token, Address, Env, String};
use storage::{load_stream, next_stream_id, save_stream};
use types::{Stream, StreamStatus};

#[contract]
pub struct StreamContract;

#[contractimpl]
impl StreamContract {
    /// Create a new payment stream.
    /// Maintainer deposits `deposit` tokens; contributor receives `rate_per_second` tokens every second
    /// for `duration` seconds.
    /// Optional `cliff_time`: timestamp before which contributor cannot withdraw (0 if none).
    /// `title`: Issue, PR, or milestone memo (e.g. "Issue #14: Indexer fix").
    pub fn create_stream(
        env: Env,
        sender: Address,
        recipient: Address,
        token: Address,
        deposit: i128,
        rate_per_second: i128,
        duration: u64,
        cliff_time: u64,
        title: String,
    ) -> Result<u64, StreamError> {
        sender.require_auth();

        if deposit <= 0 || rate_per_second <= 0 {
            return Err(StreamError::ZeroAmount);
        }
        if duration == 0 {
            return Err(StreamError::InvalidDuration);
        }
        if recipient == sender {
            return Err(StreamError::InvalidRecipient);
        }

        let required = rate_per_second * duration as i128;
        if deposit < required {
            return Err(StreamError::InsufficientBalance);
        }

        let start_time = env.ledger().timestamp();
        let stop_time = start_time + duration;

        if cliff_time != 0 && (cliff_time < start_time || cliff_time > stop_time) {
            return Err(StreamError::InvalidCliffTime);
        }

        token::Client::new(&env, &token).transfer(
            &sender,
            &env.current_contract_address(),
            &deposit,
        );

        let id = next_stream_id(&env);

        let stream = Stream {
            id,
            sender: sender.clone(),
            recipient: recipient.clone(),
            token: token.clone(),
            deposit,
            rate_per_second,
            start_time,
            stop_time,
            cliff_time,
            title: title.clone(),
            withdrawn: 0,
            elapsed_before_pause: 0,
            paused_at: 0,
            status: StreamStatus::Active,
        };

        events::stream_created(
            &env,
            id,
            &sender,
            &recipient,
            &token,
            deposit,
            rate_per_second,
            start_time,
            stop_time,
            cliff_time,
            &title,
        );
        save_stream(&env, &stream);
        Ok(id)
    }

    /// Contributor withdraws all accrued tokens available right now.
    pub fn withdraw(env: Env, stream_id: u64, recipient: Address) -> Result<i128, StreamError> {
        recipient.require_auth();

        let mut stream = load_stream(&env, stream_id).ok_or(StreamError::StreamNotFound)?;

        if stream.recipient != recipient {
            return Err(StreamError::NotRecipient);
        }
        if stream.status == StreamStatus::Cancelled || stream.status == StreamStatus::Completed {
            return Err(StreamError::AlreadyCompleted);
        }

        let now = env.ledger().timestamp();
        if stream.cliff_time > 0 && now < stream.cliff_time {
            return Err(StreamError::CliffNotReached);
        }

        let available = Self::recipient_balance_internal(&env, &stream);
        if available <= 0 {
            return Err(StreamError::InsufficientBalance);
        }

        stream.withdrawn += available;
        token::Client::new(&env, &stream.token).transfer(
            &env.current_contract_address(),
            &stream.recipient,
            &available,
        );

        if stream.withdrawn >= stream.deposit || (stream.status == StreamStatus::Active && now >= stream.stop_time) {
            stream.status = StreamStatus::Completed;
            events::stream_completed(&env, stream_id);
        }

        events::withdrawn(&env, stream_id, &stream.recipient, available);
        save_stream(&env, &stream);
        Ok(available)
    }

    /// Maintainer tops up an ongoing stream with additional tokens,
    /// extending the stop_time based on the stream's rate_per_second.
    pub fn deposit_more(
        env: Env,
        stream_id: u64,
        sender: Address,
        amount: i128,
    ) -> Result<u64, StreamError> {
        sender.require_auth();

        if amount <= 0 {
            return Err(StreamError::ZeroAmount);
        }

        let mut stream = load_stream(&env, stream_id).ok_or(StreamError::StreamNotFound)?;

        if stream.sender != sender {
            return Err(StreamError::NotSender);
        }
        if stream.status == StreamStatus::Cancelled || stream.status == StreamStatus::Completed {
            return Err(StreamError::AlreadyCompleted);
        }

        token::Client::new(&env, &stream.token).transfer(
            &sender,
            &env.current_contract_address(),
            &amount,
        );

        stream.deposit += amount;
        let additional_seconds = (amount / stream.rate_per_second) as u64;
        stream.stop_time += additional_seconds;

        events::stream_topped_up(&env, stream_id, &sender, amount, stream.stop_time);
        save_stream(&env, &stream);
        Ok(stream.stop_time)
    }

    /// Contributor transfers receiving rights to a new Stellar address (wallet rotation).
    pub fn transfer_recipient(
        env: Env,
        stream_id: u64,
        current_recipient: Address,
        new_recipient: Address,
    ) -> Result<(), StreamError> {
        current_recipient.require_auth();

        let mut stream = load_stream(&env, stream_id).ok_or(StreamError::StreamNotFound)?;

        if stream.recipient != current_recipient {
            return Err(StreamError::NotRecipient);
        }
        if stream.status == StreamStatus::Cancelled || stream.status == StreamStatus::Completed {
            return Err(StreamError::AlreadyCompleted);
        }
        if new_recipient == current_recipient || new_recipient == stream.sender {
            return Err(StreamError::InvalidRecipient);
        }

        let old_recipient = stream.recipient.clone();
        stream.recipient = new_recipient.clone();

        events::recipient_transferred(&env, stream_id, &old_recipient, &new_recipient);
        save_stream(&env, &stream);
        Ok(())
    }

    /// Maintainer cancels the stream.
    /// Contributor gets what has accrued; maintainer gets the remainder back.
    pub fn cancel_stream(env: Env, stream_id: u64, sender: Address) -> Result<(), StreamError> {
        sender.require_auth();

        let mut stream = load_stream(&env, stream_id).ok_or(StreamError::StreamNotFound)?;

        if stream.sender != sender {
            return Err(StreamError::NotSender);
        }
        if stream.status == StreamStatus::Cancelled || stream.status == StreamStatus::Completed {
            return Err(StreamError::AlreadyCompleted);
        }

        // When cancelling, accrued amount regardless of cliff is paid to contributor
        let recipient_payout = Self::vested_internal(&env, &stream);
        let sender_refund = stream.deposit - stream.withdrawn - recipient_payout;

        let token_client = token::Client::new(&env, &stream.token);

        if recipient_payout > 0 {
            token_client.transfer(
                &env.current_contract_address(),
                &stream.recipient,
                &recipient_payout,
            );
        }
        if sender_refund > 0 {
            token_client.transfer(
                &env.current_contract_address(),
                &stream.sender,
                &sender_refund,
            );
        }

        stream.status = StreamStatus::Cancelled;
        events::stream_cancelled(&env, stream_id, sender_refund, recipient_payout);
        save_stream(&env, &stream);
        Ok(())
    }

    /// Maintainer pauses the stream — stops accrual until resumed.
    pub fn pause_stream(env: Env, stream_id: u64, sender: Address) -> Result<(), StreamError> {
        sender.require_auth();

        let mut stream = load_stream(&env, stream_id).ok_or(StreamError::StreamNotFound)?;

        if stream.sender != sender {
            return Err(StreamError::NotSender);
        }
        if stream.status != StreamStatus::Active {
            return Err(StreamError::StreamNotActive);
        }

        let now = env.ledger().timestamp();
        let elapsed = now.saturating_sub(stream.start_time);
        stream.elapsed_before_pause += elapsed;
        stream.paused_at = now;
        stream.status = StreamStatus::Paused;

        events::stream_paused(&env, stream_id);
        save_stream(&env, &stream);
        Ok(())
    }

    /// Maintainer resumes a paused stream — shifts the time window forward.
    pub fn resume_stream(env: Env, stream_id: u64, sender: Address) -> Result<(), StreamError> {
        sender.require_auth();

        let mut stream = load_stream(&env, stream_id).ok_or(StreamError::StreamNotFound)?;

        if stream.sender != sender {
            return Err(StreamError::NotSender);
        }
        if stream.status != StreamStatus::Paused {
            return Err(StreamError::StreamNotPaused);
        }

        let now = env.ledger().timestamp();
        let paused_duration = now.saturating_sub(stream.paused_at);

        stream.start_time = now;
        stream.stop_time += paused_duration;
        if stream.cliff_time > 0 {
            stream.cliff_time += paused_duration;
        }
        stream.paused_at = 0;
        stream.status = StreamStatus::Active;

        events::stream_resumed(&env, stream_id);
        save_stream(&env, &stream);
        Ok(())
    }

    /// How many tokens the contributor can withdraw right now (respecting cliff).
    pub fn balance_of(env: Env, stream_id: u64) -> Result<i128, StreamError> {
        let stream = load_stream(&env, stream_id).ok_or(StreamError::StreamNotFound)?;
        Ok(Self::recipient_balance_internal(&env, &stream))
    }

    /// Total accrued tokens for the contributor so far (regardless of cliff).
    pub fn vested_of(env: Env, stream_id: u64) -> Result<i128, StreamError> {
        let stream = load_stream(&env, stream_id).ok_or(StreamError::StreamNotFound)?;
        Ok(Self::vested_internal(&env, &stream))
    }

    /// Read full stream state.
    pub fn get_stream(env: Env, stream_id: u64) -> Result<Stream, StreamError> {
        load_stream(&env, stream_id).ok_or(StreamError::StreamNotFound)
    }

    // ── Internal helpers ──────────────────────────────────────────────────────

    fn recipient_balance_internal(env: &Env, stream: &Stream) -> i128 {
        let now = env.ledger().timestamp();
        if stream.cliff_time > 0 && now < stream.cliff_time {
            return 0;
        }
        Self::vested_internal(env, stream)
    }

    fn vested_internal(env: &Env, stream: &Stream) -> i128 {
        if stream.status == StreamStatus::Paused {
            let streamed = stream.elapsed_before_pause as i128 * stream.rate_per_second;
            return (streamed - stream.withdrawn).max(0);
        }

        let now = env.ledger().timestamp();
        let effective_end = now.min(stream.stop_time);
        let elapsed = effective_end.saturating_sub(stream.start_time) as i128
            + stream.elapsed_before_pause as i128;
        let streamed = (elapsed * stream.rate_per_second).min(stream.deposit);
        (streamed - stream.withdrawn).max(0)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::{
        testutils::{Address as _, Ledger},
        token::StellarAssetClient,
        Env, String,
    };

    fn setup(env: &Env) -> (Address, Address, Address, Address) {
        env.mock_all_auths();

        let sender = Address::generate(env);
        let recipient = Address::generate(env);

        let token_admin = Address::generate(env);
        let token_id = env.register_stellar_asset_contract_v2(token_admin.clone());
        let token_addr = token_id.address();
        StellarAssetClient::new(env, &token_addr).mint(&sender, &50_000);

        let contract_id = env.register_contract(None, StreamContract);

        (sender, recipient, token_addr, contract_id)
    }

    #[test]
    fn test_create_and_withdraw() {
        let env = Env::default();
        let (sender, recipient, token_addr, contract_id) = setup(&env);
        let client = StreamContractClient::new(&env, &contract_id);
        let title = String::from_str(&env, "Issue #1: Core Feature");

        // 100 tokens/sec for 50 seconds = 5000 deposit, 0 cliff
        let stream_id = client.create_stream(&sender, &recipient, &token_addr, &5000, &100, &50, &0, &title);

        env.ledger().with_mut(|l| l.timestamp += 10);

        let available = client.balance_of(&stream_id);
        assert_eq!(available, 1000); // 10s * 100/s

        let withdrawn = client.withdraw(&stream_id, &recipient);
        assert_eq!(withdrawn, 1000);

        assert_eq!(client.balance_of(&stream_id), 0);
    }

    #[test]
    fn test_cliff_locks_and_unlocks() {
        let env = Env::default();
        let (sender, recipient, token_addr, contract_id) = setup(&env);
        let client = StreamContractClient::new(&env, &contract_id);
        let title = String::from_str(&env, "Bounty: Security Audit");

        let start_time = env.ledger().timestamp();
        let cliff_time = start_time + 20; // 20s cliff

        let stream_id = client.create_stream(&sender, &recipient, &token_addr, &5000, &100, &50, &cliff_time, &title);

        // Advance 10s (before cliff)
        env.ledger().with_mut(|l| l.timestamp += 10);
        assert_eq!(client.balance_of(&stream_id), 0); // locked by cliff
        assert_eq!(client.vested_of(&stream_id), 1000); // 1000 vested

        // Attempt withdrawal before cliff should fail
        let err = client.try_withdraw(&stream_id, &recipient);
        assert!(err.is_err());

        // Advance another 15s (25s total, past cliff)
        env.ledger().with_mut(|l| l.timestamp += 15);
        assert_eq!(client.balance_of(&stream_id), 2500);

        // Withdrawal now succeeds with all accrued tokens unlocked
        let withdrawn = client.withdraw(&stream_id, &recipient);
        assert_eq!(withdrawn, 2500);
        assert_eq!(client.balance_of(&stream_id), 0);
    }

    #[test]
    fn test_invalid_cliff_rejected() {
        let env = Env::default();
        let (sender, recipient, token_addr, contract_id) = setup(&env);
        let client = StreamContractClient::new(&env, &contract_id);
        let title = String::from_str(&env, "Invalid Cliff Stream");

        let start_time = env.ledger().timestamp();
        // Cliff past stop_time (duration is 50s)
        let bad_cliff = start_time + 60;
        let res = client.try_create_stream(&sender, &recipient, &token_addr, &5000, &100, &50, &bad_cliff, &title);
        assert!(res.is_err());
    }

    #[test]
    fn test_deposit_more_extends_stream() {
        let env = Env::default();
        let (sender, recipient, token_addr, contract_id) = setup(&env);
        let client = StreamContractClient::new(&env, &contract_id);
        let title = String::from_str(&env, "Contributor Retainer");

        // 100/s for 30s = 3000 deposit
        let stream_id = client.create_stream(&sender, &recipient, &token_addr, &3000, &100, &30, &0, &title);
        let initial_stream = client.get_stream(&stream_id);
        let original_stop = initial_stream.stop_time;

        // Top up by 2000 tokens (adds 2000 / 100 = 20s runway)
        let new_stop = client.deposit_more(&stream_id, &sender, &2000);
        assert_eq!(new_stop, original_stop + 20);

        let updated_stream = client.get_stream(&stream_id);
        assert_eq!(updated_stream.deposit, 5000);
        assert_eq!(updated_stream.stop_time, original_stop + 20);

        // Advance 40s (beyond initial 30s)
        env.ledger().with_mut(|l| l.timestamp += 40);
        assert_eq!(client.balance_of(&stream_id), 4000);
    }

    #[test]
    fn test_transfer_recipient() {
        let env = Env::default();
        let (sender, old_recipient, token_addr, contract_id) = setup(&env);
        let client = StreamContractClient::new(&env, &contract_id);
        let title = String::from_str(&env, "Dev Grant");

        let stream_id = client.create_stream(&sender, &old_recipient, &token_addr, &5000, &100, &50, &0, &title);

        env.ledger().with_mut(|l| l.timestamp += 10);

        let new_recipient = Address::generate(&env);
        client.transfer_recipient(&stream_id, &old_recipient, &new_recipient);

        let stream = client.get_stream(&stream_id);
        assert_eq!(stream.recipient, new_recipient);

        // Old recipient cannot withdraw anymore
        let err = client.try_withdraw(&stream_id, &old_recipient);
        assert!(err.is_err());

        // New recipient can withdraw
        let withdrawn = client.withdraw(&stream_id, &new_recipient);
        assert_eq!(withdrawn, 1000);
    }

    #[test]
    fn test_cancel_splits_funds() {
        let env = Env::default();
        let (sender, recipient, token_addr, contract_id) = setup(&env);
        let client = StreamContractClient::new(&env, &contract_id);
        let title = String::from_str(&env, "Milestone #1");

        client.create_stream(&sender, &recipient, &token_addr, &5000, &100, &50, &0, &title);

        env.ledger().with_mut(|l| l.timestamp += 20);

        // Cancel after 20s: recipient accrued 2000, sender gets 3000 back
        client.cancel_stream(&0, &sender);

        let stream = client.get_stream(&0);
        assert_eq!(stream.status, StreamStatus::Cancelled);
    }

    #[test]
    fn test_pause_and_resume() {
        let env = Env::default();
        let (sender, recipient, token_addr, contract_id) = setup(&env);
        let client = StreamContractClient::new(&env, &contract_id);
        let title = String::from_str(&env, "Monthly Task");

        client.create_stream(&sender, &recipient, &token_addr, &5000, &100, &50, &0, &title);

        env.ledger().with_mut(|l| l.timestamp += 10);
        client.pause_stream(&0, &sender);

        // 5 more seconds elapse while paused — balance must not increase
        env.ledger().with_mut(|l| l.timestamp += 5);
        assert_eq!(client.balance_of(&0), 1000);

        client.resume_stream(&0, &sender);
        env.ledger().with_mut(|l| l.timestamp += 10);
        assert_eq!(client.balance_of(&0), 2000);
    }

    #[test]
    fn test_full_stream_completes() {
        let env = Env::default();
        let (sender, recipient, token_addr, contract_id) = setup(&env);
        let client = StreamContractClient::new(&env, &contract_id);
        let title = String::from_str(&env, "Bounty Completion");

        client.create_stream(&sender, &recipient, &token_addr, &5000, &100, &50, &0, &title);

        env.ledger().with_mut(|l| l.timestamp += 60);

        assert_eq!(client.balance_of(&0), 5000);

        client.withdraw(&0, &recipient);
        let stream = client.get_stream(&0);
        assert_eq!(stream.status, StreamStatus::Completed);
    }

    #[test]
    fn test_wrong_recipient_cannot_withdraw() {
        let env = Env::default();
        let (sender, recipient, token_addr, contract_id) = setup(&env);
        let client = StreamContractClient::new(&env, &contract_id);
        let title = String::from_str(&env, "Auth Test");

        client.create_stream(&sender, &recipient, &token_addr, &5000, &100, &50, &0, &title);
        env.ledger().with_mut(|l| l.timestamp += 10);

        let attacker = Address::generate(&env);
        let result = client.try_withdraw(&0, &attacker);
        assert!(result.is_err());
    }

    #[test]
    fn test_recipient_cannot_be_sender() {
        let env = Env::default();
        let (sender, _, token_addr, contract_id) = setup(&env);
        let client = StreamContractClient::new(&env, &contract_id);
        let title = String::from_str(&env, "Self Stream");

        let result = client.try_create_stream(&sender, &sender, &token_addr, &5000, &100, &50, &0, &title);
        assert!(result.is_err());
    }
}
