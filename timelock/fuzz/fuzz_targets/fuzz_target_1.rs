/*

questions?

- what if different timestamps?
- what if different TimeBoundKinds?
- what if different numbers of accounts?
- deposit without approve?

- scenario:
  - generate sequence of randomized calls
  - check that balances always maintain invariants

*/

#![allow(unused)]
#![no_main]
#![feature(exclusive_range_pattern)]

use crate::arbitrary::Unstructured;
use libfuzzer_sys::fuzz_target;
use soroban_sdk::arbitrary::arbitrary::{self, Arbitrary};
use soroban_sdk::arbitrary::fuzz_catch_panic;
use soroban_sdk::arbitrary::SorobanArbitrary;
use soroban_sdk::testutils::{Ledger, LedgerInfo};
use soroban_sdk::{contractimpl, contracttype, Vec};
use soroban_sdk::{vec, Address, BytesN, Env, IntoVal};
use soroban_timelock_contract::*;

static TEST_MINT_AMOUNT: i128 = 10_000_000;

fuzz_target!(|test_case: TestCase| {
    match test_case {
        TestCase::ApproveDeposit(test) => test.run(),
        TestCase::ApproveDepositTwice(test) => test.run(),
        TestCase::Deposit(test) => test.run(),
        TestCase::DepositAboveAllowance(test) => test.run(),
        TestCase::Claim(test) => test.run(),
        TestCase::ClaimTwice(test) => test.run(),
        TestCase::UnauthorizedClaim(test) => test.run(),
        TestCase::OutOfTimeBoundClaim(test) => test.run(),
    }
});

#[derive(Arbitrary, Debug)]
enum TestCase {
    ApproveDeposit(TestApproveDeposit),
    ApproveDepositTwice(TestApproveDepositTwice),
    Deposit(TestDeposit),
    DepositAboveAllowance(TestDepositAboveAllowance),
    Claim(TestClaim),
    ClaimTwice(TestClaimTwice),
    UnauthorizedClaim(TestUnauthorizedClaim),
    OutOfTimeBoundClaim(TestOutOfTimeBoundClaim),
}

#[derive(Arbitrary, Debug)]
struct TestApproveDeposit {
    approval_amount: i128,
}

impl TestApproveDeposit {
    fn run(&self) {
        let test = ClaimableBalanceTest::setup();

        let r = fuzz_catch_panic(|| test.approve_deposit(self.approval_amount));

        if self.approval_amount < 0 {
            assert!(r.is_err());
        } else {
            assert!(r.is_ok());
        }
    }
}

#[derive(Arbitrary, Debug)]
struct TestApproveDepositTwice {
    #[arbitrary(with = |u: &mut Unstructured| u.int_in_range(0..=i128::MAX))]
    approval_amount: i128,
}

impl TestApproveDepositTwice {
    fn run(&self) {
        let mut test = ClaimableBalanceTest::setup();
        test.approve_deposit(self.approval_amount);
        let r = fuzz_catch_panic(|| test.approve_deposit(self.approval_amount));

        match self.approval_amount.checked_add(self.approval_amount) {
            Some(_) => {
                assert!(r.is_ok());
            }
            None => {
                assert!(r.is_err());
            }
        }
    }
}

#[derive(Arbitrary, Debug)]
struct TestDeposit {
    #[arbitrary(with = |u: &mut Unstructured| u.int_in_range(0..=i128::MAX))]
    approval_amount: i128,
}

impl TestDeposit {
    fn run(&self) {
        let test = ClaimableBalanceTest::setup();

        match self.approval_amount {
            0 => {
                eprintln!("TestDeposit. Approving amount 0!");
                test.approve_deposit(self.approval_amount);
                test.deposit(
                    self.approval_amount,
                    &vec![
                        &test.env,
                        Identifier::Account(test.claim_users[0].clone()),
                        Identifier::Account(test.claim_users[1].clone()),
                    ],
                    TimeBound {
                        kind: TimeBoundKind::Before,
                        timestamp: 12346,
                    },
                );
            }
            1..=i128::MAX => {
                if self.approval_amount > TEST_MINT_AMOUNT {
                    eprintln!("TestDeposit. Approving more than {}!", TEST_MINT_AMOUNT);
                    test.approve_deposit(self.approval_amount);

                    eprintln!(
                        "TestDeposit. Depositing amount more than {}!",
                        TEST_MINT_AMOUNT
                    );
                    let r = fuzz_catch_panic(|| {
                        test.deposit(
                            self.approval_amount,
                            &vec![
                                &test.env,
                                Identifier::Account(test.claim_users[0].clone()),
                                Identifier::Account(test.claim_users[1].clone()),
                            ],
                            TimeBound {
                                kind: TimeBoundKind::Before,
                                timestamp: 12346,
                            },
                        );
                    });

                    assert!(r.is_err());
                } else {
                    eprintln!(
                        "TestDeposit. Approving valid amount 0~{}.",
                        TEST_MINT_AMOUNT
                    );
                    test.approve_deposit(self.approval_amount);
                    test.deposit(
                        self.approval_amount,
                        &vec![
                            &test.env,
                            Identifier::Account(test.claim_users[0].clone()),
                            Identifier::Account(test.claim_users[1].clone()),
                        ],
                        TimeBound {
                            kind: TimeBoundKind::Before,
                            timestamp: 12346,
                        },
                    );

                    let deposit_user_balance = TEST_MINT_AMOUNT
                        .checked_sub(self.approval_amount)
                        .expect("overflow");
                    assert_eq!(test.token.balance(&test.contract_id), self.approval_amount,);
                    assert_eq!(
                        test.token.balance(&Identifier::Account(test.deposit_user)),
                        deposit_user_balance,
                    );
                }
            }
            _ => panic!(),
        }
    }
}

#[derive(Debug)]
struct TestDepositAboveAllowance {
    approval_amount: i128,
    deposit_amount: i128,
}

impl<'a> Arbitrary<'a> for TestDepositAboveAllowance {
    fn arbitrary(u: &mut Unstructured<'a>) -> arbitrary::Result<Self> {
        let approval_amount = u.int_in_range(0..=(i128::MAX - 1))?;
        let deposit_amount = u.int_in_range((approval_amount + 1)..=i128::MAX)?;
        Ok(TestDepositAboveAllowance {
            approval_amount, deposit_amount,
        })
    }
}

impl TestDepositAboveAllowance {
    fn run(&self) {
        let test = ClaimableBalanceTest::setup();

        eprintln!(
            "TestDepositAboveAllowance. Approving amount: {}",
            self.approval_amount
        );
        test.approve_deposit(self.approval_amount);

        eprintln!("TestDepositAboveAllowance. Depositing {}", self.deposit_amount);

        let r = fuzz_catch_panic(|| {
            test.deposit(
                self.deposit_amount,
                &vec![
                    &test.env,
                    Identifier::Account(test.claim_users[0].clone()),
                    Identifier::Account(test.claim_users[1].clone()),
                ],
                TimeBound {
                    kind: TimeBoundKind::Before,
                    timestamp: 12346,
                },
            );
        });

        assert!(r.is_err());
    }
}

#[derive(Arbitrary, Debug)]
struct TestClaim {
    approval_amount: i128,
}

impl TestClaim {
    fn run(&self) {
        let test = ClaimableBalanceTest::setup();

        if self.approval_amount >= 0 && self.approval_amount <= TEST_MINT_AMOUNT {
            eprintln!("TestClaim. Approve valid amount {}", self.approval_amount);

            test.approve_deposit(self.approval_amount);
            test.deposit(
                self.approval_amount,
                &vec![
                    &test.env,
                    Identifier::Account(test.claim_users[0].clone()),
                    Identifier::Account(test.claim_users[1].clone()),
                ],
                TimeBound {
                    kind: TimeBoundKind::Before,
                    timestamp: 12346,
                },
            );

            let deposit_user_balance = TEST_MINT_AMOUNT
                .checked_sub(self.approval_amount)
                .expect("overflow");
            assert_eq!(
                test.token
                    .balance(&Identifier::Account(test.deposit_user.clone())),
                deposit_user_balance,
            );

            assert_eq!(test.token.balance(&test.contract_id), self.approval_amount,);

            test.claim(&test.claim_users[1]);

            assert_eq!(
                test.token
                    .balance(&Identifier::Account(test.claim_users[1].clone())),
                self.approval_amount,
            );
        } else {
            eprintln!("TestClaim. Approve invalid amount {}", self.approval_amount);
            let r = fuzz_catch_panic(|| {
                test.approve_deposit(self.approval_amount);
                test.deposit(
                    self.approval_amount,
                    &vec![
                        &test.env,
                        Identifier::Account(test.claim_users[0].clone()),
                        Identifier::Account(test.claim_users[1].clone()),
                    ],
                    TimeBound {
                        kind: TimeBoundKind::Before,
                        timestamp: 12346,
                    },
                );

                let deposit_user_balance = TEST_MINT_AMOUNT
                    .checked_sub(self.approval_amount)
                    .expect("overflow");
                assert_eq!(
                    test.token
                        .balance(&Identifier::Account(test.deposit_user.clone())),
                    deposit_user_balance,
                );

                assert_eq!(test.token.balance(&test.contract_id), self.approval_amount,);

                test.claim(&test.claim_users[1]);

                assert_eq!(
                    test.token
                        .balance(&Identifier::Account(test.claim_users[1].clone())),
                    self.approval_amount,
                );
            });

            assert!(r.is_err());
        }
    }
}

#[derive(Arbitrary, Debug)]
struct TestClaimTwice {
    approval_amount: i128,
}

impl TestClaimTwice {
    fn run(&self) {
        let test = ClaimableBalanceTest::setup();

        if self.approval_amount >= 0 && self.approval_amount <= TEST_MINT_AMOUNT {
            eprintln!(
                "TestClaimTwice. Approve valid amount {}",
                self.approval_amount
            );

            test.approve_deposit(self.approval_amount);
            test.deposit(
                self.approval_amount,
                &vec![
                    &test.env,
                    Identifier::Account(test.claim_users[0].clone()),
                    Identifier::Account(test.claim_users[1].clone()),
                ],
                TimeBound {
                    kind: TimeBoundKind::Before,
                    timestamp: 12346,
                },
            );

            test.claim(&test.claim_users[0]);

            let r = fuzz_catch_panic(|| {
                test.claim(&test.claim_users[0]);
            });

            assert!(r.is_err());
        } else {
            eprintln!(
                "TestClaimTwice. Approve invalid amount {}",
                self.approval_amount
            );
            let r = fuzz_catch_panic(|| {
                test.approve_deposit(self.approval_amount);
                test.deposit(
                    self.approval_amount,
                    &vec![
                        &test.env,
                        Identifier::Account(test.claim_users[0].clone()),
                        Identifier::Account(test.claim_users[1].clone()),
                    ],
                    TimeBound {
                        kind: TimeBoundKind::Before,
                        timestamp: 12346,
                    },
                );
                test.claim(&test.claim_users[0]);
                test.claim(&test.claim_users[0]);
            });

            assert!(r.is_err());
        }
    }
}

#[derive(Arbitrary, Debug)]
struct TestUnauthorizedClaim {
    approval_amount: i128,
}

impl TestUnauthorizedClaim {
    fn run(&self) {
        let test = ClaimableBalanceTest::setup();

        if self.approval_amount >= 0 && self.approval_amount <= TEST_MINT_AMOUNT {
            eprintln!(
                "TestUnauthorizedClaim. Approve valid amount {}",
                self.approval_amount
            );
            test.approve_deposit(self.approval_amount);
            test.deposit(
                self.approval_amount,
                &vec![
                    &test.env,
                    Identifier::Account(test.claim_users[0].clone()),
                    Identifier::Account(test.claim_users[1].clone()),
                ],
                TimeBound {
                    kind: TimeBoundKind::Before,
                    timestamp: 12346,
                },
            );

            let r = fuzz_catch_panic(|| {
                test.claim(&test.claim_users[2]);
            });

            assert!(r.is_err());
        } else {
            eprintln!(
                "TestUnauthorizedClaim. Approve invalid amount {}",
                self.approval_amount
            );
            let r = fuzz_catch_panic(|| {
                test.approve_deposit(self.approval_amount);
                test.deposit(
                    self.approval_amount,
                    &vec![
                        &test.env,
                        Identifier::Account(test.claim_users[0].clone()),
                        Identifier::Account(test.claim_users[1].clone()),
                    ],
                    TimeBound {
                        kind: TimeBoundKind::Before,
                        timestamp: 12346,
                    },
                );

                test.claim(&test.claim_users[2]);
            });

            assert!(r.is_err());
        }
    }
}

#[derive(Arbitrary, Debug)]
struct TestOutOfTimeBoundClaim {
    approval_amount: i128,
}

impl TestOutOfTimeBoundClaim {
    fn run(&self) {
        let test = ClaimableBalanceTest::setup();

        if self.approval_amount >= 0 && self.approval_amount <= TEST_MINT_AMOUNT {
            eprintln!(
                "TestOutOfTimeBoundClaim. Approve valid amount {}",
                self.approval_amount
            );

            test.approve_deposit(self.approval_amount);
            test.deposit(
                self.approval_amount,
                &vec![&test.env, Identifier::Account(test.claim_users[0].clone())],
                TimeBound {
                    kind: TimeBoundKind::After,
                    timestamp: 12346,
                },
            );

            let r = fuzz_catch_panic(|| {
                test.claim(&test.claim_users[0]);
            });

            assert!(r.is_err());
        } else {
            eprintln!(
                "TestOutOfTimeBoundClaim. Approve invalid amount {}",
                self.approval_amount
            );

            let r = fuzz_catch_panic(|| {
                test.approve_deposit(self.approval_amount);
                test.deposit(
                    self.approval_amount,
                    &vec![&test.env, Identifier::Account(test.claim_users[0].clone())],
                    TimeBound {
                        kind: TimeBoundKind::After,
                        timestamp: 12346,
                    },
                );

                test.claim(&test.claim_users[0]);
            });

            assert!(r.is_err());
        }
    }
}

soroban_sdk::contractimport!(
    file = "../../target/wasm32-unknown-unknown/release/soroban_token_contract.wasm"
);

type TokenClient = Client;

fn create_token_contract(e: &Env, admin: &Address) -> (BytesN<32>, TokenClient) {
    e.install_contract_wasm(WASM);

    let id = e.register_contract_wasm(None, WASM);
    let token = TokenClient::new(e, &id);
    // decimals, name, symbol don't matter in tests
    token.initialize(
        &Identifier::Account(admin.clone()),
        &7u32,
        &"name".into_val(e),
        &"symbol".into_val(e),
    );
    (id, token)
}

fn create_claimable_balance_contract(e: &Env) -> ClaimableBalanceContractClient {
    ClaimableBalanceContractClient::new(e, &e.register_contract(None, ClaimableBalanceContract {}))
}

struct ClaimableBalanceTest {
    env: Env,
    deposit_address: Address,
    claim_addresses: [Address; 3],
    token: TokenClient,
    contract: ClaimableBalanceContractClient,
    contract_address: Address,
}

impl ClaimableBalanceTest {
    fn setup() -> Self {
        let env: Env = Default::default();
        env.ledger().set(LedgerInfo {
            timestamp: 12345,
            protocol_version: 1,
            sequence_number: 10,
            network_id: Default::default(),
            base_reserve: 10,
        });

        let deposit_address = Address::random(&env);

        let claim_addresses = [
            Address::random(&env),
            Address::random(&env),
            Address::random(&env),
        ];

        let token_admin = Address::random(&env);

        let token = create_token_contract(&env, &token_admin);
        token.mint(&token_admin, &deposit_address, &1000);

        let contract = create_claimable_balance_contract(&env);
        let contract_address = Address::from_contract_id(&env, &contract.contract_id);
        ClaimableBalanceTest {
            env,
            deposit_address,
            claim_addresses,
            token,
            contract,
            contract_address,
        }
    }
}
