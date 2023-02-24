#![no_main]

use libfuzzer_sys::fuzz_target;
use soroban_sdk::arbitrary::SorobanArbitrary;
use soroban_sdk::BytesN;
use soroban_sdk::{Env, IntoVal};

fuzz_target!(|input: <BytesN<34> as SorobanArbitrary>::Prototype| {
    let env = Env::default();
    let _input: BytesN<34> = input.into_val(&env);
});
