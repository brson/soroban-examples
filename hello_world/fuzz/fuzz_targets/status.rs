#![no_main]

use libfuzzer_sys::fuzz_target;
use soroban_sdk::Status;

fuzz_target!(|input: Status| {
    let _input = input.get_code();
});
