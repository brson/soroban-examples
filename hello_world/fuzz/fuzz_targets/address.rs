#![no_main]

use libfuzzer_sys::fuzz_target;
use soroban_sdk::arbitrary::SorobanArbitrary;
use soroban_sdk::Address;
use soroban_sdk::{Env, IntoVal};

fuzz_target!(|input: <Address as SorobanArbitrary>::Prototype| {
    let env = Env::default();
    let _input: Address = input.into_val(&env);
});
