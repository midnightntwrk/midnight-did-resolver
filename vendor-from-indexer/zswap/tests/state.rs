#![deny(warnings)]

use coin_structure::coin::{Info as CoinInfo, TokenType};
use coin_structure::contract::Address as ContractAddress;
use coin_structure::storage::db::{DB, InMemoryDB};
use coin_structure::transient_crypto::proofs::ProofPreimage;
use midnight_zswap::keys::SecretKeys;
use midnight_zswap::ledger::State;
use midnight_zswap::local;
use midnight_zswap::{Offer, Output as ZswapOutput};
use rand::rngs::{OsRng, StdRng};
use rand::{Rng, SeedableRng};

#[test]
fn coin_receiving() {
    let mut rng = StdRng::seed_from_u64(0x42);
    let mut state = local::State::<InMemoryDB>::new();
    let keys = SecretKeys::from_rng_seed(&mut rng);
    let coin = CoinInfo {
        nonce: OsRng.gen(),
        type_: TokenType(OsRng.gen()),
        value: OsRng.gen(),
    };
    let output = ZswapOutput::new(
        &mut rng,
        &coin,
        0,
        &keys.coin_public_key(),
        Some(keys.enc_public_key()),
    )
    .unwrap();
    let offer = Offer {
        inputs: vec![],
        outputs: vec![output],
        transient: vec![],
        deltas: vec![],
    };
    state = state.apply(&keys, &offer);
    assert_eq!(
        state
            .coins
            .iter()
            .map(|(_, c)| CoinInfo::from(&*c))
            .collect::<Vec<_>>(),
        vec![coin]
    );
}

#[test]
fn state_filtering() {
    let mut rng = StdRng::seed_from_u64(0x42);
    let mut state: State<InMemoryDB> = State::new();

    fn apply_random_offer<D: DB>(
        state: &mut State<D>,
        rng: &mut StdRng,
    ) -> ZswapOutput<ProofPreimage> {
        let coin = CoinInfo {
            nonce: OsRng.gen(),
            type_: TokenType(OsRng.gen()),
            value: OsRng.gen(),
        };
        let address = ContractAddress(OsRng.gen());
        let output = ZswapOutput::new_contract_owned(rng, &coin, 0, address).unwrap();
        let offer = Offer {
            inputs: Vec::new(),
            outputs: vec![output.clone()],
            transient: Vec::new(),
            deltas: Vec::new(),
        };

        *state = state.try_apply(&offer, None).unwrap().0;
        output
    }

    for _ in 0..2 {
        apply_random_offer(&mut state, &mut rng);
    }
    let output = apply_random_offer(&mut state, &mut rng);

    let reference_tree = state.coin_coms.collapse(0, 1);

    assert_eq!(
        state.filter(&[output.contract_address.unwrap()]),
        reference_tree
    );

    for _ in 0..2 {
        apply_random_offer(&mut state, &mut rng);
    }

    let mut reference_tree = state.coin_coms.collapse(0, 1);
    reference_tree = reference_tree.collapse(3, 4);

    assert_eq!(
        state.filter(&[output.contract_address.unwrap()]),
        reference_tree
    );
}
