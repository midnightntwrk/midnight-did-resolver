use coin_structure::coin::{Info as CoinInfo, NATIVE_TOKEN};
use lazy_static::lazy_static;
use midnight_ledger::base_crypto::rng::SplittableRng;
use midnight_ledger::semantics::{ErasedTransactionResult::Success, ZswapLocalStateExt};
use midnight_ledger::structure::{
    ClaimMintTransaction, LedgerState, MAX_SUPPLY, OutputInstruction, SystemTransaction,
    Transaction,
};
use midnight_ledger::test_utilities::{Resolver, test_resolver, tx_prove};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use zswap::keys::SecretKeys;
use zswap::local::State as ZswapLocalState;
use zswap::storage::db::InMemoryDB;

lazy_static! {
    static ref RESOLVER: Resolver = test_resolver("");
}

#[tokio::test]
async fn system_tx() {
    let mut rng = StdRng::seed_from_u64(0x42);
    // Initial states
    let mut ledger_state: LedgerState<InMemoryDB> = LedgerState::new();

    let mut zswap = ZswapLocalState::new();
    let sks = SecretKeys::from_rng_seed(&mut rng);

    // Mint
    let sys_tx = SystemTransaction::Mint(vec![OutputInstruction {
        amount: 500_000,
        target_key: sks.coin_public_key(),
    }]);
    ledger_state = ledger_state.apply_system_tx(&sys_tx).unwrap();
    let nonce = rng.gen();
    let mint_cost = ledger_state.parameters.cost_model.mint_cost as u128;
    let tx = tx_prove(
        rng.split(),
        &Transaction::ClaimMint(ClaimMintTransaction::from(
            zswap
                .authorize_mint(
                    &mut rng,
                    &sks,
                    CoinInfo {
                        value: 500_000 - mint_cost,
                        type_: NATIVE_TOKEN,
                        nonce,
                    },
                )
                .unwrap(),
        )),
        &RESOLVER,
    )
    .await
    .unwrap();
    ledger_state = ledger_state.assert_apply(&tx);
    zswap = zswap.apply_tx(&sks, &tx, Success);
    assert_eq!(
        ledger_state.unminted_native_token_supply,
        MAX_SUPPLY - 500_000
    );
    assert_eq!(ledger_state.treasury.get(&NATIVE_TOKEN), Some(&mint_cost));

    // Treasury transfer
    let sys_tx = SystemTransaction::PayFromTreasury {
        outputs: vec![OutputInstruction {
            amount: mint_cost,
            target_key: sks.coin_public_key(),
        }],
        nonce: rng.gen(),
        token_type: NATIVE_TOKEN,
    };
    ledger_state = ledger_state.apply_system_tx(&sys_tx).unwrap();
    zswap = zswap.apply_system_tx(&sks, &sys_tx);
    assert_eq!(
        ledger_state
            .treasury
            .get(&NATIVE_TOKEN)
            .copied()
            .unwrap_or(0),
        0
    );
    assert_eq!(zswap.coins.iter().map(|a| a.1.value).sum::<u128>(), 500_000);
}
