#![no_main]

use libfuzzer_sys::fuzz_target;
use soroban_sdk::arbitrary::BitSet;

fuzz_target!(|input: BitSet| {
    let _input = input.to_u64();
});
