// As this test relies on a ZK check of the segment ID.
#![cfg(feature = "proving")]

use lazy_static::lazy_static;
use midnight_ledger::coin_structure::coin::{Info as CoinInfo, NATIVE_TOKEN};
use midnight_ledger::semantics::{TransactionContext, TransactionResult};
use midnight_ledger::structure::{ContractCalls, ContractDeploy, LedgerState, Transaction};
use midnight_ledger::test_utilities::{Resolver, test_resolver, tx_prove};
use midnight_ledger::verify::WellFormedStrictness;
use midnight_ledger::zswap::{Offer, Output, local::State as ZswapLocalState};
use onchain_runtime::state::ContractState;
use rand::{Rng, SeedableRng, rngs::StdRng};
use zswap::base_crypto::rng::SplittableRng;
use zswap::keys::SecretKeys;
use zswap::storage::db::InMemoryDB;

lazy_static! {
    static ref RESOLVER: Resolver = test_resolver("");
}

#[tokio::test]
async fn malicious_imbalance() {
    let mut rng = StdRng::seed_from_u64(0x42);
    // Initial states
    let mut ledger_state: LedgerState<InMemoryDB> = LedgerState::new();
    let mut zswap_state = ZswapLocalState::<InMemoryDB>::new();
    let sks = SecretKeys::from_rng_seed(&mut rng);

    // Lets start by giving ourself 100 million tokens. We just need something to start.
    let coin = CoinInfo {
        nonce: rng.gen(),
        value: 100_000_000,
        type_: NATIVE_TOKEN,
    };
    let funding_offer = Offer {
        inputs: vec![],
        outputs: vec![
            Output::new(
                &mut rng,
                &coin,
                0,
                &sks.coin_public_key(),
                Some(sks.enc_public_key()),
            )
            .unwrap(),
        ],
        transient: vec![],
        deltas: vec![],
    };
    let deploy = ContractDeploy::new(&mut rng, ContractState::default());
    // Bypass some machinery to give it to ourselves directly.
    zswap_state = zswap_state.apply(&sks, &funding_offer);
    ledger_state.zswap = ledger_state
        .zswap
        .try_apply(&funding_offer, None)
        .unwrap()
        .0;
    // We also bypass some machinery to directly deploy an existing contract.
    ledger_state.contract = ledger_state
        .contract
        .insert(deploy.address(), deploy.initial_state.clone());

    // We are now at our genesis state. The goal of the attack is to create 95 million tokens from
    // thin air. The 5 million difference is just a generous buffer for transaction fees.
    // (It turns out even attackers pay taxes)

    // Let's create the coin that we want to create maliciously.
    let bad_coin = CoinInfo {
        nonce: rng.gen(),
        value: 95_000_000,
        type_: NATIVE_TOKEN,
    };
    // And a corresponding Zswap output
    let bad_output = Output::new(
        &mut rng,
        &bad_coin,
        0,
        &sks.coin_public_key(),
        Some(sks.enc_public_key()),
    )
    .unwrap();
    // We create an offer that gives it to ourselves -- but we claim the offer has a net *input* of
    // 5 million (to cover the fees)
    let bad_offer_guaranteed = Offer {
        inputs: vec![],
        outputs: vec![bad_output],
        transient: vec![],
        deltas: vec![(NATIVE_TOKEN, 5_000_000)],
    };
    // In order to "balance" the transaction, we create a fallible offer that spends our 100m
    // tokens. But we declare that it has *no* net input.
    let bad_offer_fallible1 = Offer {
        inputs: vec![
            zswap_state
                .spend(&mut rng, &sks, &coin.qualify(0), 1)
                .unwrap()
                .1,
        ],
        outputs: vec![],
        transient: vec![],
        deltas: vec![],
    };
    // We also test with (incorrectly) using a guaranteed offer in a fallible context
    let bad_offer_fallible2 = Offer {
        inputs: vec![
            zswap_state
                .spend(&mut rng, &sks, &coin.qualify(0), 0)
                .unwrap()
                .1,
        ],
        outputs: vec![],
        transient: vec![],
        deltas: vec![],
    };
    // We create a transaction with these two offers, and attempting to deploy the already deployed
    // contract.
    let calls = ContractCalls::new(&mut rng).add_deploy(deploy);
    for bad_offer_fallible in [bad_offer_fallible1, bad_offer_fallible2] {
        let bad_tx1 = Transaction::new(
            bad_offer_guaranteed.clone(),
            Some(bad_offer_fallible),
            Some(calls.clone()),
        );
        let bad_tx1 = tx_prove(rng.split(), &bad_tx1, &RESOLVER).await.unwrap();
        // This transaction should partially succeed application, meaning that that 95m output is
        // applied, but the 100m input is not spent, leaving us with 195m left over... from 100m start.
        assert!(matches!(
            ledger_state
                .apply(&bad_tx1, &TransactionContext::default())
                .1,
            TransactionResult::PartialSuccess(_)
        ));
        // Because of this, this transaction had damn well better not be considered a well-formed one!
        assert!(
            bad_tx1
                .well_formed(&ledger_state, WellFormedStrictness::default())
                .is_err()
        );
    }
}
