#![deny(warnings)]

use coin_structure::contract::Address as ContractAddress;
use midnight_ledger::base_crypto::signatures::SigningKey;
use midnight_ledger::serialize::{Deserializable, NetworkId, deserialize, serialize};
use midnight_ledger::storage::storage::HashMap;
use midnight_ledger::transient_crypto::proofs::{ProofPreimage as BaseProofPreimage, VerifierKey};
use midnight_ledger::{
    error::{MalformedTransaction, TransactionInvalid},
    semantics::TransactionResult,
    structure::{
        ContractCalls, ContractDeploy, ContractOperationVersion,
        ContractOperationVersionedVerifierKey, LedgerState, MaintenanceUpdate, ProofPreimage,
        SingleUpdate, Transaction,
    },
    verify::WellFormedStrictness,
};
use onchain_runtime::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf, StateValue,
};
use rand::{CryptoRng, Rng, SeedableRng, rngs::StdRng};
use zswap::Offer;
use zswap::storage::db::{DB, InMemoryDB};

fn empty_offer() -> Offer<BaseProofPreimage> {
    Offer {
        inputs: vec![],
        outputs: vec![],
        transient: vec![],
        deltas: vec![],
    }
}

fn update_tx<R: Rng + CryptoRng, D: DB>(
    rng: &mut R,
    update: MaintenanceUpdate,
) -> Transaction<ProofPreimage, D> {
    Transaction::new(
        empty_offer(),
        None,
        Some(ContractCalls::new(rng).add_maintenance_update(update.clone())),
    )
}

#[cfg(feature = "proving")]
#[tokio::test]
async fn schnorr_validity() {
    use lazy_static::lazy_static;
    use midnight_ledger::test_utilities::{Resolver, test_resolver, tx_prove};
    use zswap::base_crypto::rng::SplittableRng;

    let mut rng = StdRng::seed_from_u64(0x42);
    // Initial states
    let mut ledger_state: LedgerState<InMemoryDB> = LedgerState::new();
    let mut strictness = WellFormedStrictness::default();
    strictness.enforce_balancing = false;

    let authority = ContractMaintenanceAuthority {
        committee: vec![],
        threshold: 0,
        counter: 0,
    };
    let state = ContractState {
        data: StateValue::Null,
        operations: crate::HashMap::new(),
        maintenance_authority: authority.clone(),
    };
    let deploy = ContractDeploy::new(&mut rng, state);
    let addr = deploy.address();
    let deploy_tx = Transaction::new(
        empty_offer(),
        None,
        Some(ContractCalls::new(&mut rng).add_deploy(deploy)),
    );
    ledger_state = ledger_state.assert_apply(&deploy_tx);

    let next_authority = ContractMaintenanceAuthority {
        committee: vec![],
        threshold: 1,
        counter: 1,
    };

    let update = MaintenanceUpdate::new(
        addr,
        vec![SingleUpdate::ReplaceAuthority(next_authority.clone())],
        0,
    );

    let tx = Transaction::new(
        empty_offer(),
        None,
        Some(ContractCalls::new(&mut rng).add_maintenance_update(update.clone())),
    );
    lazy_static! {
        static ref RESOLVER: Resolver = test_resolver("");
    }
    let mut tx = tx_prove(rng.split(), &tx, &RESOLVER).await.unwrap();
    let mut tx_ser = Vec::new();
    serialize(&tx, &mut tx_ser, NetworkId::Undeployed).unwrap();
    tx = deserialize(&mut &tx_ser[..], NetworkId::Undeployed).unwrap();
    let mut tx_ser2 = Vec::new();
    serialize(&tx, &mut tx_ser2, NetworkId::Undeployed).unwrap();
    assert_eq!(tx_ser, tx_ser2);
    assert!(tx.well_formed(&ledger_state, strictness).is_ok());
}

#[test]
fn maintenance() {
    let mut rng = StdRng::seed_from_u64(0x42);
    // Initial states
    let mut ledger_state: LedgerState<InMemoryDB> = LedgerState::new();
    let mut strictness = WellFormedStrictness::default();
    strictness.enforce_balancing = false;
    let fake_vk = VerifierKey::deserialize(&mut &b"\x04\x00\x00\x00\x00\x00\x00"[..], 0).unwrap();

    let committee_sks: Vec<_> = (0..4).map(|_| SigningKey::sample(&mut rng)).collect();
    let committee_pks = committee_sks
        .iter()
        .map(SigningKey::verifying_key)
        .collect::<Vec<_>>();
    let authority = ContractMaintenanceAuthority {
        committee: committee_pks.clone(),
        threshold: 2,
        counter: 0,
    };
    let state = ContractState {
        data: StateValue::Null,
        operations: HashMap::new().insert(
            b"foo"[..].to_owned().into(),
            ContractOperation::new(Some(fake_vk.clone())),
        ),
        maintenance_authority: authority.clone(),
    };
    let deploy = ContractDeploy::new(&mut rng, state);
    let addr = deploy.address();
    let deploy_tx = Transaction::new(
        empty_offer(),
        None,
        Some(ContractCalls::new(&mut rng).add_deploy(deploy)),
    );
    ledger_state = ledger_state.assert_apply(&deploy_tx);

    let next_authority = ContractMaintenanceAuthority {
        committee: committee_pks,
        threshold: 3,
        counter: 1,
    };

    let update = MaintenanceUpdate::new(
        addr,
        vec![SingleUpdate::ReplaceAuthority(next_authority.clone())],
        0,
    );

    // Insufficient signatures
    // Then replace with sufficient signatures
    {
        let data = update.data_to_sign();
        let mut update = update
            .clone()
            .add_signature(1, committee_sks[1].sign(&mut rng, &data));

        let tx = Transaction::new(
            empty_offer(),
            None,
            Some(ContractCalls::new(&mut rng).add_maintenance_update(update.clone())),
        );
        assert!(matches!(
            dbg!(tx.well_formed(&ledger_state, strictness)),
            Err(MalformedTransaction::ThresholdMissed { .. })
        ));

        update = update.add_signature(3, committee_sks[3].sign(&mut rng, &data));
        update = update.add_signature(2, committee_sks[2].sign(&mut rng, &data));

        let mut tx = update_tx(&mut rng, update.clone());
        let mut tx_ser = Vec::new();
        serialize(&tx, &mut tx_ser, NetworkId::Undeployed).unwrap();
        tx = deserialize(&mut &tx_ser[..], NetworkId::Undeployed).unwrap();
        assert!(dbg!(tx.well_formed(&ledger_state, strictness)).is_ok());
        let state2 = ledger_state.assert_apply(&tx);
        assert_eq!(
            state2.index(addr).as_ref().unwrap().maintenance_authority,
            next_authority.clone()
        );
    }

    // Targeting the wrong contract address
    {
        let mut update = update.clone();
        update.address = ContractAddress(rng.gen());
        let data = update.data_to_sign();
        for i in 0..2 {
            update = update.add_signature(i, committee_sks[i as usize].sign(&mut rng, &data));
        }
        let tx = update_tx(&mut rng, update.clone());
        assert!(matches!(
            dbg!(tx.well_formed(&ledger_state, strictness)),
            Err(MalformedTransaction::ContractNotPresent { .. })
        ));
    }

    // Signing the wrong data
    {
        let mut data = update.data_to_sign();
        data[0] = 0;
        let mut update = update.clone();
        for i in 0..2 {
            update = update.add_signature(i, committee_sks[i as usize].sign(&mut rng, &data));
        }
        let tx = update_tx(&mut rng, update.clone());
        assert!(matches!(
            dbg!(tx.well_formed(&ledger_state, strictness)),
            Err(MalformedTransaction::InvalidCommitteeSignature { .. })
        ));
    }

    // Signing the wrong keys (key invalid)
    {
        let data = update.data_to_sign();
        let mut update = update.clone();
        for i in 0..2 {
            let key = SigningKey::sample(&mut rng);
            update = update.add_signature(i, key.sign(&mut rng, &data));
        }
        let tx = update_tx(&mut rng, update.clone());
        assert!(matches!(
            dbg!(tx.well_formed(&ledger_state, strictness)),
            Err(MalformedTransaction::InvalidCommitteeSignature { .. })
        ));
    }

    // Signing the wrong keys (key ID not in committee)
    {
        let data = update.data_to_sign();
        let mut update = update.clone();
        for i in 0..2 {
            update = update.add_signature(i + 10, committee_sks[i as usize].sign(&mut rng, &data));
        }
        let tx = update_tx(&mut rng, update.clone());
        assert!(matches!(
            dbg!(tx.well_formed(&ledger_state, strictness)),
            Err(MalformedTransaction::KeyNotInCommittee { .. })
        ));
    }

    // Signing the wrong keys (signed with valid committe key, for the wrong ID)
    {
        let data = update.data_to_sign();
        let mut update = update.clone();
        for i in 0..2 {
            update = update.add_signature(i, committee_sks[3 - i as usize].sign(&mut rng, &data));
        }
        let tx = update_tx(&mut rng, update.clone());
        dbg!(&tx);
        assert!(matches!(
            dbg!(tx.well_formed(&ledger_state, strictness)),
            Err(MalformedTransaction::InvalidCommitteeSignature { .. })
        ));
    }

    // Multi-signing
    {
        let data = update.data_to_sign();
        let mut update = update.clone();
        for _ in 0..2 {
            update = update.add_signature(0, committee_sks[0].sign(&mut rng, &data));
        }
        let tx = update_tx(&mut rng, update.clone());
        dbg!(&tx);
        assert!(matches!(
            dbg!(tx.well_formed(&ledger_state, strictness)),
            Err(MalformedTransaction::NotNormalized { .. })
        ));
    }

    // Wrong tx counter
    {
        let mut update = update.clone();
        update.counter = 1;
        let data = update.data_to_sign();
        for i in 0..2 {
            update = update.add_signature(i, committee_sks[i as usize].sign(&mut rng, &data));
        }
        let tx = update_tx(&mut rng, update.clone());
        dbg!(&tx);
        assert!(matches!(
            dbg!(tx.well_formed(&ledger_state, strictness)),
            Err(MalformedTransaction::NotNormalized { .. })
        ));
    }

    // remove + insert
    {
        let mut update = update.clone();
        update.updates = vec![
            SingleUpdate::VerifierKeyRemove(
                b"foo"[..].to_owned().into(),
                ContractOperationVersion::V2,
            ),
            SingleUpdate::VerifierKeyInsert(
                b"bar"[..].to_owned().into(),
                ContractOperationVersionedVerifierKey::V2(fake_vk.clone()),
            ),
            SingleUpdate::VerifierKeyInsert(
                b"baz"[..].to_owned().into(),
                ContractOperationVersionedVerifierKey::V2(fake_vk.clone()),
            ),
            SingleUpdate::VerifierKeyRemove(
                b"baz"[..].to_owned().into(),
                ContractOperationVersion::V2,
            ),
        ];
        let data = update.data_to_sign();
        for i in 0..2 {
            update = update.add_signature(i, committee_sks[i as usize].sign(&mut rng, &data));
        }
        let tx = update_tx(&mut rng, update.clone());
        dbg!(&tx);
        assert!(dbg!(tx.well_formed(&ledger_state, strictness)).is_ok());
        let state2 = ledger_state.assert_apply(&tx);
        let cstate = state2.index(addr).unwrap();
        assert_eq!(cstate.maintenance_authority.threshold, authority.threshold);
        assert_eq!(cstate.maintenance_authority.committee, authority.committee);
        assert_eq!(cstate.maintenance_authority.counter, authority.counter + 1);
        assert!(
            cstate
                .operations
                .get(&EntryPointBuf(b"foo"[..].to_owned()))
                .is_none()
        );
        assert!(
            cstate
                .operations
                .get(&EntryPointBuf(b"bar"[..].to_owned()))
                .is_some()
        );
        assert!(
            cstate
                .operations
                .get(&EntryPointBuf(b"baz"[..].to_owned()))
                .is_none()
        );
    }

    // remove not present
    {
        let mut update = update.clone();
        update.updates = vec![SingleUpdate::VerifierKeyRemove(
            b"bar"[..].to_owned().into(),
            ContractOperationVersion::V2,
        )];
        let data = update.data_to_sign();
        for i in 0..2 {
            update = update.add_signature(i, committee_sks[i as usize].sign(&mut rng, &data));
        }
        let tx = update_tx(&mut rng, update.clone());
        dbg!(&tx);
        assert!(dbg!(tx.well_formed(&ledger_state, strictness)).is_ok());
        assert!(matches!(
            dbg!(ledger_state.apply(&tx, &Default::default())),
            (
                _,
                TransactionResult::PartialSuccess(TransactionInvalid::VerifierKeyNotFound(..))
            )
        ));
    }

    // insert already present
    {
        let mut update = update.clone();
        update.updates = vec![SingleUpdate::VerifierKeyInsert(
            b"foo"[..].to_owned().into(),
            ContractOperationVersionedVerifierKey::V2(fake_vk.clone()),
        )];
        let data = update.data_to_sign();
        for i in 0..2 {
            update = update.add_signature(i, committee_sks[i as usize].sign(&mut rng, &data));
        }
        let tx = update_tx(&mut rng, update.clone());
        dbg!(&tx);
        assert!(dbg!(tx.well_formed(&ledger_state, strictness)).is_ok());
        assert!(matches!(
            dbg!(ledger_state.apply(&tx, &Default::default())),
            (
                _,
                TransactionResult::PartialSuccess(TransactionInvalid::VerifierKeyAlreadyPresent(
                    ..
                ))
            )
        ));
    }
}
