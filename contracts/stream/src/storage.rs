use soroban_sdk::{contracttype, Env};
use crate::types::Stream;

const DAY_IN_LEDGERS: u32 = 17280; // ~5s per ledger
const INSTANCE_EXTEND_TTL: u32 = 30 * DAY_IN_LEDGERS; // 30 days
const INSTANCE_THRESHOLD: u32 = 7 * DAY_IN_LEDGERS;
const PERSISTENT_EXTEND_TTL: u32 = 90 * DAY_IN_LEDGERS; // 90 days
const PERSISTENT_THRESHOLD: u32 = 14 * DAY_IN_LEDGERS;

#[contracttype]
pub enum DataKey {
    Stream(u64),
    NextId,
}

pub fn next_stream_id(env: &Env) -> u64 {
    bump_instance_ttl(env);
    let id: u64 = env.storage().instance().get(&DataKey::NextId).unwrap_or(0);
    env.storage().instance().set(&DataKey::NextId, &(id + 1));
    id
}

pub fn save_stream(env: &Env, stream: &Stream) {
    let key = DataKey::Stream(stream.id);
    env.storage().persistent().set(&key, stream);
    env.storage().persistent().extend_ttl(&key, PERSISTENT_THRESHOLD, PERSISTENT_EXTEND_TTL);
    bump_instance_ttl(env);
}

pub fn load_stream(env: &Env, id: u64) -> Option<Stream> {
    let key = DataKey::Stream(id);
    let stream = env.storage().persistent().get(&key);
    if stream.is_some() {
        env.storage().persistent().extend_ttl(&key, PERSISTENT_THRESHOLD, PERSISTENT_EXTEND_TTL);
    }
    bump_instance_ttl(env);
    stream
}

pub fn bump_instance_ttl(env: &Env) {
    env.storage().instance().extend_ttl(INSTANCE_THRESHOLD, INSTANCE_EXTEND_TTL);
}
