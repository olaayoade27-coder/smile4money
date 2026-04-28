# Issue #2: Prevent Double Initialization of Oracle Contract

**Labels:** `bug`  
**Priority:** High  
**Estimated Time:** 30 minutes

## Problem

`OracleContract::initialize` had no guard against being called a second time. Any caller could overwrite the admin address after deployment, taking full control of result submission.

## Root Cause

The function wrote directly to instance storage without checking whether `DataKey::Admin` already existed:

```rust
// Before fix — no guard
pub fn initialize(env: Env, admin: Address) {
    env.storage().instance().set(&DataKey::Admin, &admin);
}
```

## Fix

Check for `DataKey::Admin` before writing. Return `Error::AlreadyInitialized` if already set:

```rust
pub fn initialize(env: Env, admin: Address) -> Result<(), Error> {
    if env.storage().instance().has(&DataKey::Admin) {
        return Err(Error::AlreadyInitialized);
    }
    env.storage().instance().set(&DataKey::Admin, &admin);
    Ok(())
}
```

`AlreadyInitialized` must be added to the `Error` enum in `errors.rs`.

## Test Case

```rust
#[test]
fn test_double_initialize_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let contract_id = env.register(OracleContract, ());
    let client = OracleContractClient::new(&env, &contract_id);

    client.initialize(&admin).unwrap();
    assert_eq!(
        client.try_initialize(&admin),
        Err(Ok(Error::AlreadyInitialized))
    );
}
```

## Security Impact

Without this guard, any caller could replace the admin address post-deployment and submit fraudulent match results, redirecting payouts to arbitrary addresses.
