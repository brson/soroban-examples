// todo
//
// AdvanceTime should also reset the Env

#![allow(unused)]
#![no_main]

use libfuzzer_sys::fuzz_target;
use soroban_sdk::arbitrary::arbitrary::{self, Arbitrary, Unstructured};
use soroban_sdk::arbitrary::fuzz_catch_panic;
use soroban_sdk::arbitrary::SorobanArbitrary;
use soroban_sdk::testutils::{Address as _, Ledger, LedgerInfo};
use soroban_sdk::{contracttype, vec, Address, Env, IntoVal, Vec};
use soroban_timelock_contract::*;
use std::collections::BTreeSet;
use std::sync::Arc;
use std::vec::Vec as RustVec;

fuzz_target!(|input: TestInput| {
    let mut test = ClaimableBalanceTest::setup(input);
    let mut run_state = RunState::default();

    assert_invariants(&test);

    for test_step in test.test_input.test_steps.clone().iter() {
        match test_step {
            TestStep::Deposit(test_step) => test_step.run(&test),
            TestStep::Claim(test_step) => test_step.run(&test),
            TestStep::AdvanceTime(test_step) => test_step.run(&mut test),
        }

        make_assertions(&test, &test_step, &mut run_state);
    }
});

fn make_assertions(test: &ClaimableBalanceTest, test_step: &TestStep, run_state: &mut RunState) {
    assert_invariants(test);
    assert_stateful(test, test_step, run_state);
}

fn assert_invariants(test: &ClaimableBalanceTest) {
    let env = &test.env;

    env.as_contract(&test.contract.contract_id, || {
        let is_initialized = env.storage().has(&DataKey::Init);
        let claimable_balance = env.storage().get::<_, ClaimableBalance>(&DataKey::Balance);
        let ledger_timestamp = env.ledger().timestamp();
        let token_balance = test.token.balance(&test.contract_address);

        // make assertions

        assert!(ledger_timestamp >= test.test_input.start_timestamp);

        if !is_initialized {
            assert!(claimable_balance.is_none());
        }

        if let Some(claimable_balance) = claimable_balance {
            assert!(claimable_balance.is_ok());
            let claimable_balance = claimable_balance.expect(".");

            assert_eq!(token_balance, claimable_balance.amount);

            // todo
            assert_eq!(claimable_balance.token, test.token.contract_id);

            assert!(claimable_balance.amount <= test.test_input.mint_amount);
            assert!(claimable_balance.amount >= 0);
            // according to the contract it's ok to have 0 claimants
            //assert!(claimable_balance.claimants.len() > 0);
            //assert!(claimable_balance.claimants.len() <= 8);
            assert!(claimable_balance.claimants.len() <= 10);

            let expected_claimants: BTreeSet<Address> =
                test.claim_addresses.clone().into_iter().collect();
            let actual_claimants: Result<BTreeSet<Address>, _> =
                claimable_balance.claimants.clone().into_iter().collect();
            let actual_claimants = actual_claimants.expect(".");
            assert_eq!(expected_claimants, actual_claimants);
        } else {
            assert!(token_balance == 0);
        }
    });
}

fn assert_stateful(test: &ClaimableBalanceTest, test_step: &TestStep, run_state: &mut RunState) {
    let env = &test.env;

    env.as_contract(&test.contract.contract_id, || {
        let is_initialized = env.storage().has(&DataKey::Init);
        let balance = env.storage().get::<_, ClaimableBalance>(&DataKey::Balance);

        // make assertions

        if run_state.has_seen_init {
            assert!(is_initialized);
        }

        if is_initialized {
            // on init balance is set
            // on claim it is unset
            if !run_state.has_seen_init {
                assert!(balance.is_some());
            }
        }

        // update run_state

        run_state.has_seen_init = run_state.has_seen_init || is_initialized;
    });
}

#[derive(Default)]
struct RunState {
    has_seen_init: bool,
}

#[derive(Debug)]
struct TestInput {
    start_timestamp: u64,
    //#[arbitrary(with = |u: &mut Unstructured| u.int_in_range(0..=i128::MAX))]
    mint_amount: i128,
    claim_addresses: RustVec<<Address as SorobanArbitrary>::Prototype>,
    nonclaim_addresses: RustVec<<Address as SorobanArbitrary>::Prototype>,
    test_steps: Arc<RustVec<TestStep>>,
}

impl<'a> Arbitrary<'a> for TestInput {
    fn arbitrary(u: &mut Unstructured<'a>) -> arbitrary::Result<Self> {
        let start_timestamp = u.arbitrary()?;
        let mint_amount = u.int_in_range(0..=i128::MAX)?;
        let test_steps = u.arbitrary()?;

        let addrs: BTreeSet<<Address as SorobanArbitrary>::Prototype> = u.arbitrary()?;
        let mut addrs: RustVec<_> = addrs.into_iter().collect();
        let nonclaim_addresses = addrs.pop().into_iter().collect();
        let claim_addresses = addrs;

        Ok(TestInput {
            start_timestamp,
            mint_amount,
            claim_addresses,
            nonclaim_addresses,
            test_steps,
        })
    }
}

#[derive(Arbitrary, Debug)]
enum TestStep {
    Deposit(StepDeposit),
    Claim(StepClaim),
    AdvanceTime(StepAdvanceTime),
}

#[derive(Arbitrary, Debug)]
struct StepDeposit {
    deposit_amount: i128,
    time_bound: <TimeBound as SorobanArbitrary>::Prototype,
}

#[derive(Arbitrary, Debug)]
struct StepClaim {
    claimant_index: usize,
}

#[derive(Arbitrary, Debug)]
struct StepAdvanceTime {
    amount: u64,
}

impl StepDeposit {
    fn run(&self, test: &ClaimableBalanceTest) {
        let time_bound: TimeBound = self.time_bound.into_val(&test.env);

        let addrs = Vec::from_slice(&test.env, &test.claim_addresses);

        let r = fuzz_catch_panic(|| {
            test.contract.deposit(
                &test.deposit_address,
                &test.token.contract_id,
                &self.deposit_amount,
                &addrs,
                &time_bound,
            )
        });

        let env = &test.env;
        env.as_contract(&test.contract.contract_id, || {
            let balance = env.storage().get::<_, ClaimableBalance>(&DataKey::Balance);

            if r.is_ok() {
                assert!(balance.is_some());
            }
        });
    }
}

impl StepClaim {
    fn run(&self, test: &ClaimableBalanceTest) {
        let env = &test.env;

        if test.all_addresses.is_empty() {
            // No claimants
            return;
        }

        let claimant_index = self.claimant_index % test.all_addresses.len();
        let claim_address = &test.all_addresses[claimant_index];

        let pre_balance = test.token.balance(&test.contract_address);
        let timestamp = env.ledger().timestamp();
        let pre_claimable_balance = env.as_contract(&test.contract.contract_id, || {
            env.storage().get::<_, ClaimableBalance>(&DataKey::Balance)
        });

        let r = fuzz_catch_panic(|| {
            test.contract.claim(claim_address);
        });

        let post_balance = test.token.balance(&test.contract_address);

        env.as_contract(&test.contract.contract_id, || {
            let claimable_balance = env.storage().get::<_, ClaimableBalance>(&DataKey::Balance);

            if r.is_ok() {
                assert!(claimable_balance.is_none());
                assert_eq!(post_balance, 0);
            } else {
                assert_eq!(post_balance, pre_balance);
            }
        });

        // Only succeed for valid claimants
        if r.is_ok() {
            assert!(test.claim_addresses.contains(&claim_address));
        }

        // Claim can't succeed outside the timelock bounds
        if let Some(Ok(pre_claimable_balance)) = pre_claimable_balance {
            let time_bound = pre_claimable_balance.time_bound;
            match time_bound.kind {
                TimeBoundKind::Before => {
                    if timestamp > time_bound.timestamp {
                        assert!(r.is_err());
                    }
                }
                TimeBoundKind::After => {
                    if timestamp < time_bound.timestamp {
                        assert!(r.is_err());
                    }
                }
            }
        }
    }
}

impl StepAdvanceTime {
    fn run(&self, test: &mut ClaimableBalanceTest) {
        test.reset_env_after_advance_time(self.amount);
    }
}

mod token {
    soroban_sdk::contractimport!(file = "../../soroban_token_spec.wasm");
    pub type TokenClient = Client;
}

use token::TokenClient;

fn create_token_contract(e: &Env, admin: &Address) -> TokenClient {
    TokenClient::new(e, &e.register_stellar_asset_contract(admin.clone()))
}

fn create_claimable_balance_contract(e: &Env) -> ClaimableBalanceContractClient {
    ClaimableBalanceContractClient::new(e, &e.register_contract(None, ClaimableBalanceContract {}))
}

struct ClaimableBalanceTest {
    test_input: TestInput,
    env: Env,
    claim_addresses: RustVec<Address>,
    nonclaim_addresses: RustVec<Address>,
    // claimaints + non-claimaints
    all_addresses: RustVec<Address>,
    deposit_address: Address,
    token: TokenClient,
    contract: ClaimableBalanceContractClient,
    contract_address: Address,
}

impl ClaimableBalanceTest {
    fn setup(test_input: TestInput) -> Self {
        let env: Env = Default::default();
        env.ledger().set(LedgerInfo {
            timestamp: test_input.start_timestamp,
            protocol_version: 1,
            sequence_number: 10,
            network_id: Default::default(),
            base_reserve: 10,
        });

        env.budget().reset_unlimited();

        let claim_addresses: RustVec<_> = test_input
            .claim_addresses
            .iter()
            .map(|a| a.into_val(&env))
            .collect();
        let nonclaim_addresses: RustVec<_> = test_input
            .nonclaim_addresses
            .iter()
            .map(|a| a.into_val(&env))
            .collect();
        let all_addresses: RustVec<_> = claim_addresses
            .iter()
            .chain(nonclaim_addresses.iter())
            .cloned()
            .collect();

        let deposit_address = Address::random(&env);

        let token_admin = Address::random(&env);

        let token = create_token_contract(&env, &token_admin);
        token.mint(&token_admin, &deposit_address, &test_input.mint_amount);

        let contract = create_claimable_balance_contract(&env);
        let contract_address = Address::from_contract_id(&env, &contract.contract_id);
        ClaimableBalanceTest {
            test_input,
            env,
            claim_addresses,
            nonclaim_addresses,
            all_addresses,
            deposit_address,
            token,
            contract,
            contract_address,
        }
    }

    fn reset_env_after_advance_time(&mut self, amount: u64) -> &mut Self {
        self.env.ledger().with_mut(|ledger| {
            ledger.sequence_number = ledger.sequence_number.saturating_add(1);
            ledger.timestamp = ledger.timestamp.saturating_add(amount);
        });

        self.env.budget().reset_unlimited();

        self
    }
}
