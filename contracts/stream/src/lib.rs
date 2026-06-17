#![no_std]

mod errors;
mod events;
mod storage;
mod types;

use errors::StreamError;
use soroban_sdk::{contract, contractimpl, token, Address, Env};
use storage::{load_stream, next_stream_id, save_stream};
use types::{Stream, StreamStatus};

#[contract]
pub struct StreamContract;

#[contractimpl]
impl StreamContract {
    /// Create a new payment stream.
    /// Sender deposits `deposit` tokens; recipient receives `rate_per_second` tokens every second
    /// for `duration` seconds.
    pub fn create_stream(
        env: Env,
        sender: Address,
        recipient: Address,
        token: Address,
        deposit: i128,
        rate_per_second: i128,
        duration: u64,
    ) -> Result<u64, StreamError> {
        sender.require_auth();

        if deposit <= 0 || rate_per_second <= 0 {
            return Err(StreamError::ZeroAmount);
        }
        if duration == 0 {
            return Err(StreamError::InvalidDuration);
        }

        let required = rate_per_second * duration as i128;
        if deposit < required {
            return Err(StreamError::InsufficientBalance);
        }

        token::Client::new(&env, &token).transfer(
            &sender,
            &env.current_contract_address(),
            &deposit,
        );

        let start_time = env.ledger().timestamp();
        let stop_time = start_time + duration;
        let id = next_stream_id(&env);

        let stream = Stream {
            id,
            sender,
            recipient,
            token,
            deposit,
            rate_per_second,
            start_time,
            stop_time,
            withdrawn: 0,
            elapsed_before_pause: 0,
            paused_at: 0,
            status: StreamStatus::Active,
        };

        events::stream_created(&env, id, &stream.sender, &stream.recipient, deposit, rate_per_second, start_time, stop_time);
        save_stream(&env, &stream);
        Ok(id)
    }

    /// Recipient withdraws all tokens streamed so far.
    pub fn withdraw(env: Env, stream_id: u64, recipient: Address) -> Result<i128, StreamError> {
        recipient.require_auth();

        let mut stream = load_stream(&env, stream_id).ok_or(StreamError::StreamNotFound)?;

        if stream.recipient != recipient {
            return Err(StreamError::NotRecipient);
        }
        if stream.status == StreamStatus::Cancelled || stream.status == StreamStatus::Completed {
            return Err(StreamError::AlreadyCompleted);
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

        let now = env.ledger().timestamp();
        if stream.status == StreamStatus::Active && now >= stream.stop_time {
            stream.status = StreamStatus::Completed;
        }

        events::withdrawn(&env, stream_id, &stream.recipient, available);
        save_stream(&env, &stream);
        Ok(available)
    }

    /// Sender cancels the stream.
    /// Recipient gets what has accrued; sender gets the remainder back.
    pub fn cancel_stream(env: Env, stream_id: u64, sender: Address) -> Result<(), StreamError> {
        sender.require_auth();

        let mut stream = load_stream(&env, stream_id).ok_or(StreamError::StreamNotFound)?;

        if stream.sender != sender {
            return Err(StreamError::NotSender);
        }
        if stream.status == StreamStatus::Cancelled || stream.status == StreamStatus::Completed {
            return Err(StreamError::AlreadyCompleted);
        }

        let recipient_payout = Self::recipient_balance_internal(&env, &stream);
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

    /// Sender pauses the stream — stops accrual until resumed.
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
        // Snapshot how many seconds have elapsed so far.
        let elapsed = now.saturating_sub(stream.start_time);
        stream.elapsed_before_pause += elapsed;
        stream.paused_at = now;
        stream.status = StreamStatus::Paused;

        events::stream_paused(&env, stream_id);
        save_stream(&env, &stream);
        Ok(())
    }

    /// Sender resumes a paused stream — shifts the time window forward.
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

        // Shift the stream window forward by however long it was paused.
        stream.start_time = now;
        stream.stop_time += paused_duration;
        stream.paused_at = 0;
        stream.status = StreamStatus::Active;

        events::stream_resumed(&env, stream_id);
        save_stream(&env, &stream);
        Ok(())
    }

    /// How many tokens the recipient can withdraw right now.
    pub fn balance_of(env: Env, stream_id: u64) -> Result<i128, StreamError> {
        let stream = load_stream(&env, stream_id).ok_or(StreamError::StreamNotFound)?;
        Ok(Self::recipient_balance_internal(&env, &stream))
    }

    /// Read full stream state.
    pub fn get_stream(env: Env, stream_id: u64) -> Result<Stream, StreamError> {
        load_stream(&env, stream_id).ok_or(StreamError::StreamNotFound)
    }

    // ── Internal helpers ──────────────────────────────────────────────────────

    fn recipient_balance_internal(env: &Env, stream: &Stream) -> i128 {
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
        Env,
    };

    fn setup(env: &Env) -> (Address, Address, Address, Address) {
        env.mock_all_auths();

        let sender = Address::generate(env);
        let recipient = Address::generate(env);

        let token_admin = Address::generate(env);
        let token_id = env.register_stellar_asset_contract_v2(token_admin.clone());
        let token_addr = token_id.address();
        StellarAssetClient::new(env, &token_addr).mint(&sender, &10_000);

        let contract_id = env.register_contract(None, StreamContract);

        (sender, recipient, token_addr, contract_id)
    }

    #[test]
    fn test_create_and_withdraw() {
        let env = Env::default();
        let (sender, recipient, token_addr, contract_id) = setup(&env);
        let client = StreamContractClient::new(&env, &contract_id);

        // 100 tokens/sec for 50 seconds = 5000 deposit
        let stream_id = client.create_stream(&sender, &recipient, &token_addr, &5000, &100, &50);

        env.ledger().with_mut(|l| l.timestamp += 10);

        let available = client.balance_of(&stream_id);
        assert_eq!(available, 1000); // 10s * 100/s

        let withdrawn = client.withdraw(&stream_id, &recipient);
        assert_eq!(withdrawn, 1000);

        assert_eq!(client.balance_of(&stream_id), 0);
    }

    #[test]
    fn test_cancel_splits_funds() {
        let env = Env::default();
        let (sender, recipient, token_addr, contract_id) = setup(&env);
        let client = StreamContractClient::new(&env, &contract_id);

        client.create_stream(&sender, &recipient, &token_addr, &5000, &100, &50);

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

        client.create_stream(&sender, &recipient, &token_addr, &5000, &100, &50);

        env.ledger().with_mut(|l| l.timestamp += 10);
        client.pause_stream(&0, &sender);

        // 5 more seconds elapse while paused — balance must not increase
        env.ledger().with_mut(|l| l.timestamp += 5);
        assert_eq!(client.balance_of(&0), 1000); // still 10s * 100

        client.resume_stream(&0, &sender);
        env.ledger().with_mut(|l| l.timestamp += 10);
        assert_eq!(client.balance_of(&0), 2000); // 20s total active time
    }

    #[test]
    fn test_full_stream_completes() {
        let env = Env::default();
        let (sender, recipient, token_addr, contract_id) = setup(&env);
        let client = StreamContractClient::new(&env, &contract_id);

        client.create_stream(&sender, &recipient, &token_addr, &5000, &100, &50);

        env.ledger().with_mut(|l| l.timestamp += 60);

        // Balance capped at full deposit even though more time elapsed
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

        client.create_stream(&sender, &recipient, &token_addr, &5000, &100, &50);
        env.ledger().with_mut(|l| l.timestamp += 10);

        let attacker = Address::generate(&env);
        let result = client.try_withdraw(&0, &attacker);
        assert!(result.is_err());
    }
}
