use soroban_sdk::contracterror;

#[contracterror]
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(u32)]
pub enum StreamError {
    StreamNotFound = 1,
    NotSender = 2,
    NotRecipient = 3,
    StreamNotActive = 4,
    StreamNotPaused = 5,
    ZeroAmount = 6,
    InvalidDuration = 7,
    InsufficientBalance = 8,
    AlreadyCompleted = 9,
}
