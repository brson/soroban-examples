#![no_main]

use libfuzzer_sys::fuzz_target;
use soroban_sdk::arbitrary::SorobanArbitrary;
use soroban_sdk::{Bytes, Env, IntoVal, Vec};

fuzz_target!(|input: <Vec<Vec<Bytes>> as SorobanArbitrary>::Prototype| {
    let env = Env::default();
    let _input: Vec<Vec<Bytes>> = input.into_val(&env);
});
