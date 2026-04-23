use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Error {
    MatchNotFound = 1,
    AlreadyFunded = 2,
    NotFunded = 3,
    Unauthorized = 4,
    InvalidState = 5,
    AlreadyExists = 6,
    AlreadyInitialized = 7,
    Overflow = 8,
    ContractPaused = 9,
    InvalidAmount = 10,
    InvalidGameId = 11,
    /// player1 and player2 are the same address
    InvalidPlayers = 12,
    /// game_id has already been used in another match
    DuplicateGameId = 13,
    /// oracle submitted a result whose game_id does not match the stored match game_id
    GameIdMismatch = 14,
}
