#![no_main]

use libfuzzer_sys::fuzz_target;
use soroban_custom_types_contract::State;
use soroban_sdk::{Env, IntoVal};
use soroban_sdk::arbitrary::SorobanArbitrary;

fuzz_target!(|input: <State as SorobanArbitrary>::Prototype| {
    let env = Env::default();
    let _input: State = input.into_val(&env);
});
