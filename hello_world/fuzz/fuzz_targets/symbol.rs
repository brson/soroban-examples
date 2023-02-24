#![no_main]

use libfuzzer_sys::fuzz_target;
use soroban_hello_world_contract::*;
use soroban_sdk::{symbol, vec, Env, Symbol};

fuzz_target!(|to: Symbol| {
    let env = Env::default();
    let contract_id = env.register_contract(None, HelloContract);
    let client = HelloContractClient::new(&env, &contract_id);

    let words = client.hello(&to);
    assert_eq!(words, vec![&env, symbol!("Hello"), to]);
});
