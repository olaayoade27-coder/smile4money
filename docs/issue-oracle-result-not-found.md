# Issue: OracleContract::get_result Returns Error::ResultNotFound for Unknown Match

**Labels:** `test`  
**Priority:** Medium  
**Estimated Time:** 15 minutes

## Problem

`OracleContract::get_result` was not explicitly tested for the case where no result has been submitted for a given `match_id`. Without this test, a regression could silently change the error type or cause a panic instead of returning the expected contract error.

## Expected Behaviour

Calling `get_result(999)` on a freshly initialized oracle (no results submitted) must return `Err(Error::ResultNotFound)`.

## Root Cause

`get_result` reads from persistent storage and maps a missing key to `Error::ResultNotFound`:

```rust
pub fn get_result(env: Env, match_id: u64) -> Result<ResultEntry, Error> {
    env.storage()
        .persistent()
        .get(&DataKey::Result(match_id))
        .ok_or(Error::ResultNotFound)
}
```

This is correct, but the behaviour was untested.

## Test Case

Added to `contracts/oracle/src/lib.rs`:

```rust
#[test]
fn test_get_result_not_found() {
    let (env, contract_id) = setup();
    let client = OracleContractClient::new(&env, &contract_id);
    assert!(matches!(
        client.try_get_result(&999u64),
        Err(Ok(Error::ResultNotFound))
    ));
}
```

`try_get_result` is the fallible SDK-generated variant. `Err(Ok(...))` is the Soroban convention: outer `Err` = contract returned an error, inner `Ok` = it is a known `#[contracterror]` variant.

## Files Changed

- `contracts/oracle/src/lib.rs` — added `test_get_result_not_found`
