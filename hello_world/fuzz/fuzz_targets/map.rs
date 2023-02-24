#![no_main]

use libfuzzer_sys::fuzz_target;
use soroban_sdk::arbitrary::SorobanArbitrary;
use soroban_sdk::{Bytes, Env, IntoVal, Map};

fuzz_target!(
    |input: <Map<Bytes, Bytes> as SorobanArbitrary>::Prototype| {
        let env = Env::default();
        let _input: Map<Bytes, Bytes> = input.into_val(&env);
    }
);
