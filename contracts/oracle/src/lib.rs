#![no_std]

mod errors;
mod types;

use errors::Error;
use soroban_sdk::{contract, contractimpl, symbol_short, Address, Env, IntoVal, String, Symbol};
use types::{DataKey, MatchResult, ResultEntry};

/// ~30 days at 5s/ledger.
const MATCH_TTL_LEDGERS: u32 = 518_400;

/// Maximum allowed byte length for a game_id string.
const MAX_GAME_ID_LEN: u32 = 64;

#[contract]
pub struct OracleContract;

#[contractimpl]
impl OracleContract {
    /// Initialize with a trusted admin (the off-chain oracle service).
    pub fn initialize(env: Env, admin: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("Contract already initialized");
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.events()
            .publish((Symbol::new(&env, "oracle"), symbol_short!("init")), admin);
    }

    /// Admin submits a verified match result on-chain.
    pub fn submit_result(
        env: Env,
        match_id: u64,
        game_id: String,
        result: MatchResult,
    ) -> Result<(), Error> {
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::Unauthorized)?;
        admin.require_auth();

        if game_id.len() > MAX_GAME_ID_LEN {
            return Err(Error::InvalidGameId);
        }

        if env.storage().persistent().has(&DataKey::Result(match_id)) {
            return Err(Error::AlreadySubmitted);
        }

        env.storage().persistent().set(
            &DataKey::Result(match_id),
            &ResultEntry {
                game_id,
                result: result.clone(),
            },
        );
        env.storage().persistent().extend_ttl(
            &DataKey::Result(match_id),
            MATCH_TTL_LEDGERS,
            MATCH_TTL_LEDGERS,
        );

        env.events().publish(
            (Symbol::new(&env, "oracle"), symbol_short!("result")),
            (match_id, result),
        );

        Ok(())
    }

    /// Retrieve the stored result for a match.
    pub fn get_result(env: Env, match_id: u64) -> Result<ResultEntry, Error> {
        env.storage()
            .persistent()
            .get(&DataKey::Result(match_id))
            .ok_or(Error::ResultNotFound)
    }

    /// Check whether a result has been submitted for a match.
    pub fn has_result(env: Env, match_id: u64) -> bool {
        env.storage().persistent().has(&DataKey::Result(match_id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{
        testutils::{storage::Persistent as _, Address as _},
        Address, Env, IntoVal, String,
    };

    fn setup() -> (Env, Address) {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let contract_id = env.register(OracleContract, ());
        let client = OracleContractClient::new(&env, &contract_id);
        client.initialize(&admin);
        (env, contract_id)
    }

    #[test]
    fn test_submit_and_get_result() {
        let (env, contract_id) = setup();
        let client = OracleContractClient::new(&env, &contract_id);

        assert!(!client.has_result(&0u64));

        client.submit_result(&0u64, &String::from_str(&env, "abc123"), &MatchResult::Player1Wins);

        assert!(client.has_result(&0u64));
        assert_eq!(client.get_result(&0u64).result, MatchResult::Player1Wins);

        // TTL must be extended
        let ttl = env.as_contract(&contract_id, || {
            env.storage().persistent().get_ttl(&DataKey::Result(0u64))
        });
        assert_eq!(ttl, crate::MATCH_TTL_LEDGERS);
    }

    #[test]
    fn test_get_result_not_found() {
        let (env, contract_id) = setup();
        let client = OracleContractClient::new(&env, &contract_id);
        assert!(matches!(client.try_get_result(&999u64), Err(Ok(Error::ResultNotFound))));
    }

    #[test]
    fn test_non_admin_cannot_submit_result() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let non_admin = Address::generate(&env);
        let contract_id = env.register(OracleContract, ());
        let client = OracleContractClient::new(&env, &contract_id);
        client.initialize(&admin);

        use soroban_sdk::testutils::{MockAuth, MockAuthInvoke};
        env.set_auths(&[MockAuth {
            address: &non_admin,
            invoke: &MockAuthInvoke {
                contract: &contract_id,
                fn_name: "submit_result",
                args: (0u64, String::from_str(&env, "game"), MatchResult::Player1Wins).into_val(&env),
                sub_invokes: &[],
            },
        }.into()]);

        assert!(client.try_submit_result(&0u64, &String::from_str(&env, "game"), &MatchResult::Player1Wins).is_err());
    }

    #[test]
    #[should_panic(expected = "Contract already initialized")]
    fn test_double_initialize_fails() {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let contract_id = env.register(OracleContract, ());
        let client = OracleContractClient::new(&env, &contract_id);
        client.initialize(&admin);
        client.initialize(&admin);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #2)")]
    fn test_duplicate_submit_fails() {
        let (env, contract_id) = setup();
        let client = OracleContractClient::new(&env, &contract_id);
        client.submit_result(&0u64, &String::from_str(&env, "abc123"), &MatchResult::Draw);
        client.submit_result(&0u64, &String::from_str(&env, "abc123"), &MatchResult::Draw);
    }

    #[test]
    fn test_has_result_false_for_non_existent() {
        let (env, contract_id) = setup();
        let client = OracleContractClient::new(&env, &contract_id);
        assert!(!client.has_result(&999u64));
    }

    #[test]
    fn test_initialize_emits_event() {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let contract_id = env.register(OracleContract, ());
        let client = OracleContractClient::new(&env, &contract_id);
        client.initialize(&admin);

        let events = env.events().all();
        let topics = vec![
            &env,
            Symbol::new(&env, "oracle").into_val(&env),
            soroban_sdk::symbol_short!("init").into_val(&env),
        ];
        let matched = events.iter().find(|(_, t, _)| *t == topics);
        assert!(matched.is_some());
    }
