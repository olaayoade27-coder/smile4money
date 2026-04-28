# Issue #1: Prevent Double Initialization of Escrow Contract

**Labels:** `bug`  
**Priority:** High  
**Estimated Time:** 30 minutes

## Problem

`EscrowContract::initialize` had no guard against being called a second time. A subsequent call would silently overwrite the trusted oracle and admin addresses, allowing an attacker to substitute a malicious oracle and take control of all future payouts.

## Root Cause

The function wrote directly to instance storage without first checking whether the keys already existed:

```rust
// Before fix — no guard
pub fn initialize(env: Env, oracle: Address, admin: Address) {
    env.storage().instance().set(&DataKey::Oracle, &oracle);
    env.storage().instance().set(&DataKey::Admin, &admin);
}
```

## Fix

Check for the presence of `DataKey::Oracle` before writing. Panic with a descriptive message if the contract is already initialized:

```rust
pub fn initialize(env: Env, oracle: Address, admin: Address) {
    if env.storage().instance().has(&DataKey::Oracle) {
        panic!("Contract already initialized");
    }
    env.storage().instance().set(&DataKey::Oracle, &oracle);
    env.storage().instance().set(&DataKey::Admin, &admin);
    env.storage().instance().set(&DataKey::MatchCount, &0u64);
    env.storage().instance().set(&DataKey::Paused, &false);
}
```

## Test Case

```rust
#[test]
#[should_panic(expected = "Contract already initialized")]
fn test_double_initialize_panics() {
    let env = Env::default();
    let contract_id = env.register_contract(None, EscrowContract);
    let client = EscrowContractClient::new(&env, &contract_id);

    let oracle = Address::generate(&env);
    let admin = Address::generate(&env);

    client.initialize(&oracle, &admin);
    client.initialize(&oracle, &admin); // must panic
}
```

## Security Impact

Without this guard, any caller could overwrite the oracle address post-deployment, redirecting all payout authorization to an address they control. This is a critical trust assumption for the entire escrow system.
