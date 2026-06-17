use soroban_sdk::{symbol_short, Address, Env};

pub fn stream_created(
    env: &Env,
    stream_id: u64,
    sender: &Address,
    recipient: &Address,
    deposit: i128,
    rate_per_second: i128,
    start_time: u64,
    stop_time: u64,
) {
    env.events().publish(
        (symbol_short!("CREATED"), stream_id),
        (sender.clone(), recipient.clone(), deposit, rate_per_second, start_time, stop_time),
    );
}

pub fn withdrawn(env: &Env, stream_id: u64, recipient: &Address, amount: i128) {
    env.events().publish(
        (symbol_short!("WITHDRAW"), stream_id),
        (recipient.clone(), amount),
    );
}

pub fn stream_cancelled(env: &Env, stream_id: u64, sender_refund: i128, recipient_payout: i128) {
    env.events().publish(
        (symbol_short!("CANCEL"), stream_id),
        (sender_refund, recipient_payout),
    );
}

pub fn stream_paused(env: &Env, stream_id: u64) {
    env.events().publish(
        (symbol_short!("PAUSED"), stream_id),
        (),
    );
}

pub fn stream_resumed(env: &Env, stream_id: u64) {
    env.events().publish(
        (symbol_short!("RESUMED"), stream_id),
        (),
    );
}
