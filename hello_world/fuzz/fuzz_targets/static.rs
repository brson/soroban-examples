#![no_main]

use libfuzzer_sys::fuzz_target;
use soroban_sdk::xdr::ScStatic;
use soroban_sdk::arbitrary::Static;

fuzz_target!(|input: Static| {
    let _input_0 = input.is_type(ScStatic::Void);
    let _input_1 = input.is_type(ScStatic::True);
    let _input_2 = input.is_type(ScStatic::False);
    let _input_3 = input.is_type(ScStatic::LedgerKeyContractCode);
});
