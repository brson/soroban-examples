#![no_main]

use soroban_hello_world_contract::*;
use soroban_sdk::{vec, Env, Symbol, TryFromVal};
use stellar_xdr::ScSymbol;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|to: ScSymbol| {
    let env = Env::default();
    let contract_id = env.register_contract(None, HelloContract);
    let client = HelloContractClient::new(&env, &contract_id);

    let to = Symbol::try_from_val(&env, &to).expect("symbol");

    let words = client.hello(&to);
    assert_eq!(words, vec![&env, Symbol::short("Hello"), to]);
});
