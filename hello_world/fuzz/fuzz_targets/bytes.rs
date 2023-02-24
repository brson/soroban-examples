#![no_main]

use libfuzzer_sys::fuzz_target;
use soroban_sdk::arbitrary::SorobanArbitrary;
use soroban_sdk::Bytes;
use soroban_sdk::{Env, IntoVal};

fuzz_target!(|input: <Bytes as SorobanArbitrary>::Prototype| {
    let env = Env::default();
    let _input: Bytes = input.into_val(&env);
});
