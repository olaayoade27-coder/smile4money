#![cfg(test)]

use super::*;
use soroban_sdk::{
    testutils::{storage::Persistent as _, Address as _, Events},
    token::{Client as TokenClient, StellarAssetClient},
    vec, Address, Env, IntoVal, String, Symbol, TryFromVal,
};

fn setup() -> (Env, Address, Address, Address, Address, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let oracle = Address::generate(&env);
    let player1 = Address::generate(&env);
    let player2 = Address::generate(&env);

    let token_id = env.register_stellar_asset_contract_v2(admin.clone());
    let token_addr = token_id.address();
    let asset_client = StellarAssetClient::new(&env, &token_addr);
    asset_client.mint(&player1, &1000);
    asset_client.mint(&player2, &1000);

    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);
    client.initialize(&oracle, &admin);

    (env, contract_id, oracle, player1, player2, token_addr, admin)
}

#[test]
fn test_create_match() {
    let (env, contract_id, _oracle, player1, player2, token, _admin) = setup();
    let client = EscrowContractClient::new(&env, &contract_id);

    let id = client.create_match(
        &player1, &player2, &100, &token,
        &String::from_str(&env, "abc123"), &Platform::Lichess,
    );

    assert_eq!(id, 0);
    let m = client.get_match(&id);
    assert_eq!(m.state, MatchState::Pending);
    assert_eq!(m.created_ledger, env.ledger().sequence());
}

#[test]
fn test_get_match_not_found() {
    let (env, contract_id, ..) = setup();
    let client = EscrowContractClient::new(&env, &contract_id);
    assert!(matches!(client.try_get_match(&999), Err(Ok(Error::MatchNotFound))));
}

#[test]
fn test_deposit_and_activate() {
    let (env, contract_id, _oracle, player1, player2, token, _admin) = setup();
    let client = EscrowContractClient::new(&env, &contract_id);
    let token_client = TokenClient::new(&env, &token);

    let id = client.create_match(
        &player1, &player2, &100, &token,
        &String::from_str(&env, "abc123"), &Platform::Lichess,
    );

    client.deposit(&id, &player1);
    assert!(!client.is_funded(&id));
    client.deposit(&id, &player2);
    assert!(client.is_funded(&id));
    assert_eq!(client.get_escrow_balance(&id), 200);
    assert_eq!(client.get_match(&id).state, MatchState::Active);
    assert_eq!(token_client.balance(&player1), 900);
    assert_eq!(token_client.balance(&player2), 900);
}

#[test]
fn test_payout_winner() {
    let (env, contract_id, oracle, player1, player2, token, _admin) = setup();
    let client = EscrowContractClient::new(&env, &contract_id);
    let token_client = TokenClient::new(&env, &token);

    let id = client.create_match(
        &player1, &player2, &100, &token,
        &String::from_str(&env, "game1"), &Platform::Lichess,
    );
    client.deposit(&id, &player1);
    client.deposit(&id, &player2);
    client.submit_result(&id, &String::from_str(&env, "game1"), &Winner::Player1, &oracle);

    assert_eq!(token_client.balance(&player1), 1100);
    assert_eq!(token_client.balance(&player2), 900);
    assert_eq!(client.get_match(&id).state, MatchState::Completed);
    assert_eq!(client.get_escrow_balance(&id), 0);
}

#[test]
fn test_payout_winner_player2() {
    let (env, contract_id, oracle, player1, player2, token, _admin) = setup();
    let client = EscrowContractClient::new(&env, &contract_id);
    let token_client = TokenClient::new(&env, &token);

    let id = client.create_match(
        &player1, &player2, &100, &token,
        &String::from_str(&env, "game_player2"), &Platform::Lichess,
    );
    client.deposit(&id, &player1);
    client.deposit(&id, &player2);
    client.submit_result(&id, &String::from_str(&env, "game_player2"), &Winner::Player2, &oracle);

    assert_eq!(token_client.balance(&player1), 900);
    assert_eq!(token_client.balance(&player2), 1100);
    assert_eq!(client.get_match(&id).state, MatchState::Completed);
    assert_eq!(client.get_escrow_balance(&id), 0);
}

#[test]
fn test_draw_refund() {
    let (env, contract_id, oracle, player1, player2, token, _admin) = setup();
    let client = EscrowContractClient::new(&env, &contract_id);
    let token_client = TokenClient::new(&env, &token);

    let id = client.create_match(
        &player1, &player2, &100, &token,
        &String::from_str(&env, "game2"), &Platform::ChessDotCom,
    );
    client.deposit(&id, &player1);
    client.deposit(&id, &player2);
    client.submit_result(&id, &String::from_str(&env, "game2"), &Winner::Draw, &oracle);

    assert_eq!(token_client.balance(&player1), 1000);
    assert_eq!(token_client.balance(&player2), 1000);
}

#[test]
fn test_cancel_refunds_depositor() {
    let (env, contract_id, _oracle, player1, player2, token, _admin) = setup();
    let client = EscrowContractClient::new(&env, &contract_id);
    let token_client = TokenClient::new(&env, &token);

    let id = client.create_match(
        &player1, &player2, &100, &token,
        &String::from_str(&env, "game3"), &Platform::Lichess,
    );
    client.deposit(&id, &player1);
    client.cancel_match(&id, &player1);

    assert_eq!(token_client.balance(&player1), 1000);
    assert_eq!(token_client.balance(&player2), 1000);
    assert_eq!(client.get_match(&id).state, MatchState::Cancelled);
    assert_eq!(client.get_escrow_balance(&id), 0);
}

#[test]
fn test_player2_can_cancel_pending_match() {
    let (env, contract_id, _oracle, player1, player2, token, _admin) = setup();
    let client = EscrowContractClient::new(&env, &contract_id);
    let token_client = TokenClient::new(&env, &token);

    let id = client.create_match(
        &player1, &player2, &100, &token,
        &String::from_str(&env, "p2cancel"), &Platform::Lichess,
    );
    client.deposit(&id, &player2);
    client.cancel_match(&id, &player2);

    assert_eq!(token_client.balance(&player2), 1000);
    assert_eq!(client.get_match(&id).state, MatchState::Cancelled);
}

#[test]
fn test_cancel_active_match_fails() {
    let (env, contract_id, _oracle, player1, player2, token, _admin) = setup();
    let client = EscrowContractClient::new(&env, &contract_id);

    let id = client.create_match(
        &player1, &player2, &100, &token,
        &String::from_str(&env, "active_cancel"), &Platform::Lichess,
    );
    client.deposit(&id, &player1);
    client.deposit(&id, &player2);

    assert_eq!(client.try_cancel_match(&id, &player1), Err(Ok(Error::InvalidState)));
}

#[test]
fn test_cancel_completed_match_fails() {
    let (env, contract_id, oracle, player1, player2, token, _admin) = setup();
    let client = EscrowContractClient::new(&env, &contract_id);

    let id = client.create_match(
        &player1, &player2, &100, &token,
        &String::from_str(&env, "completed_cancel"), &Platform::Lichess,
    );
    client.deposit(&id, &player1);
    client.deposit(&id, &player2);
    client.submit_result(&id, &String::from_str(&env, "completed_cancel"), &Winner::Player1, &oracle);

    assert_eq!(client.try_cancel_match(&id, &player1), Err(Ok(Error::InvalidState)));
}

#[test]
fn test_deposit_into_completed_match_fails() {
    let (env, contract_id, oracle, player1, player2, token, _admin) = setup();
    let client = EscrowContractClient::new(&env, &contract_id);

    let id = client.create_match(
        &player1, &player2, &100, &token,
        &String::from_str(&env, "completed_deposit"), &Platform::Lichess,
    );
    client.deposit(&id, &player1);
    client.deposit(&id, &player2);
    client.submit_result(&id, &String::from_str(&env, "completed_deposit"), &Winner::Player1, &oracle);

    assert_eq!(client.try_deposit(&id, &player1), Err(Ok(Error::InvalidState)));
}

#[test]
fn test_deposit_into_cancelled_match_fails() {
    let (env, contract_id, _oracle, player1, player2, token, _admin) = setup();
    let client = EscrowContractClient::new(&env, &contract_id);

    let id = client.create_match(
        &player1, &player2, &100, &token,
        &String::from_str(&env, "cancelled_deposit"), &Platform::Lichess,
    );
    client.cancel_match(&id, &player1);

    assert_eq!(client.try_deposit(&id, &player1), Err(Ok(Error::InvalidState)));
}

#[test]
fn test_non_oracle_cannot_submit_result() {
    let (env, contract_id, _oracle, player1, player2, token, _admin) = setup();
    let client = EscrowContractClient::new(&env, &contract_id);

    let id = client.create_match(
        &player1, &player2, &100, &token,
        &String::from_str(&env, "unauth_oracle"), &Platform::Lichess,
    );
    client.deposit(&id, &player1);
    client.deposit(&id, &player2);

    let impostor = Address::generate(&env);
    assert_eq!(
        client.try_submit_result(&id, &String::from_str(&env, "unauth_oracle"), &Winner::Player1, &impostor),
        Err(Ok(Error::Unauthorized))
    );
}

#[test]
fn test_submit_result_on_pending_match_fails() {
    let (env, contract_id, oracle, player1, player2, token, _admin) = setup();
    let client = EscrowContractClient::new(&env, &contract_id);

    let id = client.create_match(
        &player1, &player2, &100, &token,
        &String::from_str(&env, "pending_submit"), &Platform::Lichess,
    );
    assert_eq!(
        client.try_submit_result(&id, &String::from_str(&env, "pending_submit"), &Winner::Player1, &oracle),
        Err(Ok(Error::InvalidState))
    );
}

#[test]
fn test_submit_result_on_completed_match_fails() {
    let (env, contract_id, oracle, player1, player2, token, _admin) = setup();
    let client = EscrowContractClient::new(&env, &contract_id);

    let id = client.create_match(
        &player1, &player2, &100, &token,
        &String::from_str(&env, "double_submit"), &Platform::Lichess,
    );
    client.deposit(&id, &player1);
    client.deposit(&id, &player2);
    client.submit_result(&id, &String::from_str(&env, "double_submit"), &Winner::Player1, &oracle);

    assert_eq!(
        client.try_submit_result(&id, &String::from_str(&env, "double_submit"), &Winner::Player2, &oracle),
        Err(Ok(Error::InvalidState))
    );
}

#[test]
fn test_submit_result_wrong_game_id_fails() {
    let (env, contract_id, oracle, player1, player2, token, _admin) = setup();
    let client = EscrowContractClient::new(&env, &contract_id);

    let id = client.create_match(
        &player1, &player2, &100, &token,
        &String::from_str(&env, "real_game"), &Platform::Lichess,
    );
    client.deposit(&id, &player1);
    client.deposit(&id, &player2);

    assert_eq!(
        client.try_submit_result(&id, &String::from_str(&env, "wrong_game"), &Winner::Player1, &oracle),
        Err(Ok(Error::GameIdMismatch))
    );
}

#[test]
#[should_panic(expected = "Contract already initialized")]
fn test_double_initialize_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let oracle = Address::generate(&env);
    let admin = Address::generate(&env);
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);
    client.initialize(&oracle, &admin);
    client.initialize(&oracle, &admin);
}

#[test]
#[should_panic(expected = "Error(Contract, #10)")]
fn test_create_match_zero_stake_fails() {
    let (env, contract_id, _oracle, player1, player2, token, _admin) = setup();
    let client = EscrowContractClient::new(&env, &contract_id);
    client.create_match(
        &player1, &player2, &0, &token,
        &String::from_str(&env, "zero_stake"), &Platform::Lichess,
    );
}

#[test]
fn test_create_match_self_match_fails() {
    let (env, contract_id, _oracle, player1, _player2, token, _admin) = setup();
    let client = EscrowContractClient::new(&env, &contract_id);
    assert_eq!(
        client.try_create_match(
            &player1, &player1, &100, &token,
            &String::from_str(&env, "self_match"), &Platform::Lichess,
        ),
        Err(Ok(Error::InvalidPlayers))
    );
}

#[test]
fn test_duplicate_game_id_rejected() {
    let (env, contract_id, _oracle, player1, player2, token, _admin) = setup();
    let client = EscrowContractClient::new(&env, &contract_id);
    let player3 = Address::generate(&env);
    let player4 = Address::generate(&env);

    client.create_match(
        &player1, &player2, &100, &token,
        &String::from_str(&env, "dup_game"), &Platform::Lichess,
    );

    assert_eq!(
        client.try_create_match(
            &player3, &player4, &100, &token,
            &String::from_str(&env, "dup_game"), &Platform::Lichess,
        ),
        Err(Ok(Error::DuplicateGameId))
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #4)")]
fn test_unauthorized_player_cannot_cancel() {
    let (env, contract_id, _oracle, player1, player2, token, _admin) = setup();
    let client = EscrowContractClient::new(&env, &contract_id);
    let id = client.create_match(
        &player1, &player2, &100, &token,
        &String::from_str(&env, "unauth_cancel"), &Platform::Lichess,
    );
    client.cancel_match(&id, &Address::generate(&env));
}

#[test]
fn test_deposit_by_non_player_returns_unauthorized() {
    let (env, contract_id, _oracle, player1, player2, token, _admin) = setup();
    let client = EscrowContractClient::new(&env, &contract_id);

    let id = client.create_match(
        &player1, &player2, &100, &token,
        &String::from_str(&env, "unauth_deposit"), &Platform::Lichess,
    );
    let stranger = Address::generate(&env);
    assert_eq!(
        client.try_deposit(&id, &stranger),
        Err(Ok(Error::Unauthorized))
    );
}

#[test]
fn test_is_funded_false_after_one_deposit() {
    let (env, contract_id, _oracle, player1, player2, token, _admin) = setup();
    let client = EscrowContractClient::new(&env, &contract_id);

    let id = client.create_match(
        &player1, &player2, &100, &token,
        &String::from_str(&env, "one_deposit"), &Platform::Lichess,
    );
    client.deposit(&id, &player1);
    assert!(!client.is_funded(&id));
    client.deposit(&id, &player2);
    assert!(client.is_funded(&id));
}

#[test]
fn test_escrow_balance_stages() {
    let (env, contract_id, _oracle, player1, player2, token, _admin) = setup();
    let client = EscrowContractClient::new(&env, &contract_id);

    let id = client.create_match(
        &player1, &player2, &100, &token,
        &String::from_str(&env, "balance_stages"), &Platform::Lichess,
    );
    assert_eq!(client.get_escrow_balance(&id), 0);
    client.deposit(&id, &player1);
    assert_eq!(client.get_escrow_balance(&id), 100);
    client.deposit(&id, &player2);
    assert_eq!(client.get_escrow_balance(&id), 200);
}

#[test]
fn test_draw_payout_exact_amounts() {
    let (env, contract_id, oracle, player1, player2, token, _admin) = setup();
    let client = EscrowContractClient::new(&env, &contract_id);
    let token_client = TokenClient::new(&env, &token);

    let id = client.create_match(
        &player1, &player2, &100, &token,
        &String::from_str(&env, "draw_exact"), &Platform::Lichess,
    );
    client.deposit(&id, &player1);
    client.deposit(&id, &player2);
    client.submit_result(&id, &String::from_str(&env, "draw_exact"), &Winner::Draw, &oracle);

    assert_eq!(token_client.balance(&player1), 1000);
    assert_eq!(token_client.balance(&player2), 1000);
    assert_eq!(client.get_escrow_balance(&id), 0);
}

#[test]
fn test_update_oracle() {
    let (env, contract_id, oracle, player1, player2, token, _admin) = setup();
    let client = EscrowContractClient::new(&env, &contract_id);
    let new_oracle = Address::generate(&env);

    let id = client.create_match(
        &player1, &player2, &100, &token,
        &String::from_str(&env, "oracle_rotate"), &Platform::Lichess,
    );
    client.deposit(&id, &player1);
    client.deposit(&id, &player2);

    client.update_oracle(&new_oracle);

    // Old oracle should now be rejected
    assert_eq!(
        client.try_submit_result(&id, &String::from_str(&env, "oracle_rotate"), &Winner::Player1, &oracle),
        Err(Ok(Error::Unauthorized))
    );
    // New oracle should succeed
    client.submit_result(&id, &String::from_str(&env, "oracle_rotate"), &Winner::Player1, &new_oracle);
    assert_eq!(client.get_match(&id).state, MatchState::Completed);
}

#[test]
fn test_pause_blocks_create_and_submit() {
    let (env, contract_id, oracle, player1, player2, token, _admin) = setup();
    let client = EscrowContractClient::new(&env, &contract_id);

    let id = client.create_match(
        &player1, &player2, &100, &token,
        &String::from_str(&env, "paused_game"), &Platform::Lichess,
    );
    client.deposit(&id, &player1);
    client.deposit(&id, &player2);

    client.pause();

    assert_eq!(
        client.try_create_match(
            &player1, &player2, &100, &token,
            &String::from_str(&env, "paused2"), &Platform::Lichess,
        ),
        Err(Ok(Error::ContractPaused))
    );
    assert_eq!(
        client.try_submit_result(&id, &String::from_str(&env, "paused_game"), &Winner::Player1, &oracle),
        Err(Ok(Error::ContractPaused))
    );

    client.unpause();
    let id2 = client.create_match(
        &player1, &player2, &100, &token,
        &String::from_str(&env, "unpaused_game"), &Platform::Lichess,
    );
    assert_eq!(id2, 1);
}

#[test]
fn test_non_admin_cannot_pause() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let non_admin = Address::generate(&env);
    let oracle = Address::generate(&env);
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);
    client.initialize(&oracle, &admin);

    use soroban_sdk::testutils::{MockAuth, MockAuthInvoke};
    env.set_auths(&[MockAuth {
        address: &non_admin,
        invoke: &MockAuthInvoke {
            contract: &contract_id,
            fn_name: "pause",
            args: ().into_val(&env),
            sub_invokes: &[],
        },
    }
    .into()]);

    assert!(client.try_pause().is_err());
}

#[test]
fn test_non_admin_cannot_update_oracle() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let non_admin = Address::generate(&env);
    let oracle = Address::generate(&env);
    let new_oracle = Address::generate(&env);
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);
    client.initialize(&oracle, &admin);

    use soroban_sdk::testutils::{MockAuth, MockAuthInvoke};
    env.set_auths(&[MockAuth {
        address: &non_admin,
        invoke: &MockAuthInvoke {
            contract: &contract_id,
            fn_name: "update_oracle",
            args: (new_oracle.clone(),).into_val(&env),
            sub_invokes: &[],
        },
    }
    .into()]);

    assert!(client.try_update_oracle(&new_oracle).is_err());
}

#[test]
fn test_ttl_extended_on_state_changes() {
    let (env, contract_id, oracle, player1, player2, token, _admin) = setup();
    let client = EscrowContractClient::new(&env, &contract_id);

    let id = client.create_match(
        &player1, &player2, &100, &token,
        &String::from_str(&env, "ttl_game"), &Platform::Lichess,
    );

    let check_ttl = |key: DataKey| {
        env.as_contract(&contract_id, || {
            env.storage().persistent().get_ttl(&key)
        })
    };

    assert_eq!(check_ttl(DataKey::Match(id)), crate::MATCH_TTL_LEDGERS);

    client.deposit(&id, &player1);
    assert_eq!(check_ttl(DataKey::Match(id)), crate::MATCH_TTL_LEDGERS);

    client.deposit(&id, &player2);
    assert_eq!(check_ttl(DataKey::Match(id)), crate::MATCH_TTL_LEDGERS);

    client.submit_result(&id, &String::from_str(&env, "ttl_game"), &Winner::Player2, &oracle);
    assert_eq!(check_ttl(DataKey::Match(id)), crate::MATCH_TTL_LEDGERS);
}

#[test]
fn test_create_match_emits_event() {
    let (env, contract_id, _oracle, player1, player2, token, _admin) = setup();
    let client = EscrowContractClient::new(&env, &contract_id);

    let id = client.create_match(
        &player1, &player2, &100, &token,
        &String::from_str(&env, "game_ev"), &Platform::Lichess,
    );

    let events = env.events().all();
    let topics = vec![
        &env,
        Symbol::new(&env, "match").into_val(&env),
        soroban_sdk::symbol_short!("created").into_val(&env),
    ];
    let matched = events.iter().find(|(_, t, _)| *t == topics);
    assert!(matched.is_some());

    let (_, _, data) = matched.unwrap();
    let (ev_id, ev_p1, ev_p2, ev_stake): (u64, Address, Address, i128) =
        TryFromVal::try_from_val(&env, &data).unwrap();
    assert_eq!((ev_id, ev_p1, ev_p2, ev_stake), (id, player1, player2, 100));
}

#[test]
fn test_deposit_emits_event() {
    let (env, contract_id, _oracle, player1, player2, token, _admin) = setup();
    let client = EscrowContractClient::new(&env, &contract_id);

    let id = client.create_match(
        &player1, &player2, &100, &token,
        &String::from_str(&env, "deposit_ev"), &Platform::Lichess,
    );
    client.deposit(&id, &player1);

    let events = env.events().all();
    let topics = vec![
        &env,
        Symbol::new(&env, "match").into_val(&env),
        soroban_sdk::symbol_short!("deposit").into_val(&env),
    ];
    let matched = events.iter().find(|(_, t, _)| *t == topics);
    assert!(matched.is_some());

    let (_, _, data) = matched.unwrap();
    let (ev_id, ev_player): (u64, Address) =
        TryFromVal::try_from_val(&env, &data).unwrap();
    assert_eq!((ev_id, ev_player), (id, player1));
}

#[test]
fn test_submit_result_emits_event() {
    let (env, contract_id, oracle, player1, player2, token, _admin) = setup();
    let client = EscrowContractClient::new(&env, &contract_id);

    let id = client.create_match(
        &player1, &player2, &100, &token,
        &String::from_str(&env, "result_ev"), &Platform::Lichess,
    );
    client.deposit(&id, &player1);
    client.deposit(&id, &player2);
    client.submit_result(&id, &String::from_str(&env, "result_ev"), &Winner::Player1, &oracle);

    let events = env.events().all();
    let topics = vec![
        &env,
        Symbol::new(&env, "match").into_val(&env),
        soroban_sdk::symbol_short!("completed").into_val(&env),
    ];
    let matched = events.iter().find(|(_, t, _)| *t == topics);
    assert!(matched.is_some());

    let (_, _, data) = matched.unwrap();
    let (ev_id, ev_winner): (u64, Winner) =
        TryFromVal::try_from_val(&env, &data).unwrap();
    assert_eq!((ev_id, ev_winner), (id, Winner::Player1));
}

#[test]
fn test_cancel_match_emits_event() {
    let (env, contract_id, _oracle, player1, player2, token, _admin) = setup();
    let client = EscrowContractClient::new(&env, &contract_id);

    let id = client.create_match(
        &player1, &player2, &100, &token,
        &String::from_str(&env, "cancel_ev"), &Platform::Lichess,
    );
    client.cancel_match(&id, &player1);

    let events = env.events().all();
    let topics = vec![
        &env,
        Symbol::new(&env, "match").into_val(&env),
        soroban_sdk::symbol_short!("cancelled").into_val(&env),
    ];
    let matched = events.iter().find(|(_, t, _)| *t == topics);
    assert!(matched.is_some());

    let (_, _, data) = matched.unwrap();
    let ev_id: u64 = TryFromVal::try_from_val(&env, &data).unwrap();
    assert_eq!(ev_id, id);
}

// Issue #59: Test that pause() prevents match creation
#[test]
fn test_pause_prevents_match_creation() {
    let (env, contract_id, _oracle, player1, player2, token, admin) = setup();
    let client = EscrowContractClient::new(&env, &contract_id);

    client.pause(&admin);

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        client.create_match(
            &player1, &player2, &100, &token,
            &String::from_str(&env, "paused_match"), &Platform::Lichess,
        )
    }));

    assert!(result.is_err());
}

// Issue #60: Test that unpause() re-enables match creation
#[test]
fn test_unpause_enables_match_creation() {
    let (env, contract_id, _oracle, player1, player2, token, admin) = setup();
    let client = EscrowContractClient::new(&env, &contract_id);

    client.pause(&admin);
    client.unpause(&admin);

    let id = client.create_match(
        &player1, &player2, &100, &token,
        &String::from_str(&env, "unpaused_match"), &Platform::Lichess,
    );

    assert_eq!(id, 0);
    let m = client.get_match(&id);
    assert_eq!(m.state, MatchState::Pending);
}

// Issue #61: Test that update_oracle() successfully rotates the oracle address
#[test]
fn test_update_oracle_rotates_address() {
    let (env, contract_id, oracle, player1, player2, token, admin) = setup();
    let client = EscrowContractClient::new(&env, &contract_id);
    let new_oracle = Address::generate(&env);

    let id = client.create_match(
        &player1, &player2, &100, &token,
        &String::from_str(&env, "oracle_test"), &Platform::Lichess,
    );
    client.deposit(&id, &player1);
    client.deposit(&id, &player2);

    client.update_oracle(&new_oracle, &admin);

    let result = client.submit_result(
        &id, &String::from_str(&env, "oracle_test"), &Winner::Player1, &new_oracle
    );

    assert_eq!(result, ());
}

// Issue #62: Test that non-admin cannot call pause(), unpause(), or update_oracle()
#[test]
fn test_non_admin_cannot_call_admin_functions() {
    let (env, contract_id, _oracle, player1, _player2, _token, _admin) = setup();
    let client = EscrowContractClient::new(&env, &contract_id);

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        client.pause(&player1)
    }));
    assert!(result.is_err());

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        client.unpause(&player1)
    }));
    assert!(result.is_err());

    let new_oracle = Address::generate(&env);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        client.update_oracle(&new_oracle, &player1)
    }));
    assert!(result.is_err());
}
