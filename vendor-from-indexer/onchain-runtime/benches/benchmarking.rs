#![deny(warnings)]

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use midnight_onchain_runtime::ops::*;
use midnight_onchain_runtime::state::*;
use midnight_onchain_runtime::storage::base_crypto::fab::AlignedValue;
use midnight_onchain_runtime::storage::storage::{Array, HashMap};
use midnight_onchain_runtime::test_utilities::run_program;
use midnight_onchain_runtime::transient_crypto::merkle_tree::MerkleTree;
use midnight_onchain_runtime::vm_value::{ValueStrength, VmValue};
use onchain_vm::runtime_state::stval;
use onchain_vm::storage::db::InMemoryDB;
use onchain_vm::vmval;
use onchain_vm::{key, op, ops, ops_int};
use std::sync::Arc;

pub fn scaling_vm_instructions(c: &mut Criterion) {
    let increments: Vec<usize> = (0..16).map(|x| 1 << x).collect();

    for input_size in 0..15 {
        c.bench_function(format!("size_{}", input_size).as_str(), |b| {
            let vmval: [VmValue<InMemoryDB>; 1] = [vmval!([(4u64); input_size])];
            let ops = ops![size];
            b.iter(|| run_program(black_box(&vmval), black_box(&ops)).unwrap())
        });
    }

    for input_size in &increments {
        c.bench_function(format!("log_{}", input_size).as_str(), |b| {
            let vmval: [VmValue<InMemoryDB>; 1] = [vmval!({0u32 => null}; *input_size)];
            let ops = ops![log];
            b.iter(|| run_program(black_box(&vmval), black_box(&ops)).unwrap())
        });
    }

    for input_size in &increments[1..] {
        c.bench_function(format!("concat_{}", input_size).as_str(), |b| {
            let vmval: Vec<VmValue<InMemoryDB>> = vec![vmval!((5u64)); *input_size];
            let ops = ops![concat(2 * (*input_size as u32))];
            b.iter(|| run_program(black_box(&vmval), black_box(&ops)).unwrap())
        });
    }

    for input_size in &increments[1..] {
        c.bench_function(format!("rem_{}", input_size).as_str(), |b| {
            // A map of length `input_size`, where each entry is the length-one
            // map `{10 => null}`.
            let vmval: [VmValue<InMemoryDB>; 1] = [vmval!({0u32 => {10u32 => null}}; *input_size)];
            // Lookup the inner map at key `0` in the outer map, and then remove
            // the entry at key `10` from that inner map.
            let ops = ops![idx [0u32]; push (10u32); rem];
            b.iter(|| run_program(black_box(&vmval), black_box(&ops)).unwrap())
        });
    }

    for input_size in &increments[1..] {
        c.bench_function(format!("ins_{}", input_size).as_str(), |b| {
            let vmval: [VmValue<InMemoryDB>; 1] = [vmval!({0u32 => null}; *input_size)];
            // Read and re-insert the entry with key `0`.
            let ops = ops![idxp [0u32]; ins 1];
            b.iter(|| run_program(black_box(&vmval), black_box(&ops)).unwrap())
        });
    }

    for input_size in &increments[1..] {
        c.bench_function(format!("idx_{}", input_size).as_str(), |b| {
            let vmval: [VmValue<InMemoryDB>; 1] = [vmval!({0u32 => null}; *input_size)];
            let ops = ops![idx[0u32]];
            b.iter(|| run_program(black_box(&vmval), black_box(&ops)).unwrap())
        });
    }

    for input_size in &increments[1..] {
        c.bench_function(format!("member_{}", input_size).as_str(), |b| {
            // A map of size `input_size` where each entry is a unit map `{10 =>
            // null}`.
            let vmval: [VmValue<InMemoryDB>; 1] = [vmval!({0u32 => {10u32 => null}}; *input_size)];
            // Lookup the inner map at index `0`, then check if `16` is a key in
            // the that map (it's not).
            let ops = ops![idx [0u32]; push (16u32); member];
            b.iter(|| run_program(black_box(&vmval), black_box(&ops)).unwrap())
        });
    }

    for input_size in &increments[1..] {
        c.bench_function(format!("push_{}", input_size).as_str(), |b| {
            // A pointless large value on the stack.
            let vmval: [VmValue<InMemoryDB>; 1] = [vmval!({0u32 => null}; *input_size)];
            // Push a small value onto the stack. Has nothing to do with
            // existing stack content ...
            let ops = ops![push(2u64)];
            b.iter(|| run_program(black_box(&vmval), black_box(&ops)).unwrap())
        });
    }
}

pub fn individual_vm_instructions(c: &mut Criterion) {
    let empty_mtree = vmval!([[{MT(8) {}}, (0u64)]]);

    c.bench_function("noop", |b| {
        let vmval: [VmValue<InMemoryDB>; 1] = [vmval!([null])];
        let ops = ops![noop(0)];
        b.iter(|| run_program(black_box(&vmval), black_box(&ops)).unwrap())
    });

    c.bench_function("lt", |b| {
        let vmval: [VmValue<InMemoryDB>; 2] = [vmval!((4u64)), vmval!((5u64))];
        let ops = ops![lt];
        b.iter(|| run_program(black_box(&vmval), black_box(&ops)).unwrap())
    });

    c.bench_function("eq", |b| {
        let vmval: [VmValue<InMemoryDB>; 2] = [vmval!((4u64)), vmval!((5u64))];
        let ops = ops![eq];
        b.iter(|| run_program(black_box(&vmval), black_box(&ops)).unwrap())
    });

    c.bench_function("type", |b| {
        let vmval: [VmValue<InMemoryDB>; 1] = [vmval!([(4u64)])];
        let ops = ops![type];
        b.iter(|| run_program(black_box(&vmval), black_box(&ops)).unwrap())
    });

    c.bench_function("size", |b| {
        let vmval: [VmValue<InMemoryDB>; 1] = [vmval!([(4u64)])];
        let ops = ops![size];
        b.iter(|| run_program(black_box(&vmval), black_box(&ops)).unwrap())
    });

    c.bench_function("new", |b| {
        let vmval: [VmValue<InMemoryDB>; 1] = [vmval!((4u64))];
        let ops = ops![new];
        b.iter(|| run_program(black_box(&vmval), black_box(&ops)).unwrap())
    });

    c.bench_function("and", |b| {
        let vmval: [VmValue<InMemoryDB>; 2] = [vmval!((true)), vmval!((false))];
        let ops = ops![and];
        b.iter(|| run_program(black_box(&vmval), black_box(&ops)).unwrap())
    });

    c.bench_function("or", |b| {
        let vmval: [VmValue<InMemoryDB>; 2] = [vmval!((true)), vmval!((false))];
        let ops = ops![or];
        b.iter(|| run_program(black_box(&vmval), black_box(&ops)).unwrap())
    });

    c.bench_function("neg", |b| {
        let vmval: [VmValue<InMemoryDB>; 1] = [vmval!((true))];
        let ops = ops![neg];
        b.iter(|| run_program(black_box(&vmval), black_box(&ops)).unwrap())
    });

    c.bench_function("log", |b| {
        let vmval: [VmValue<InMemoryDB>; 1] = [vmval!((4u64))];
        let ops = ops![log];
        b.iter(|| run_program(black_box(&vmval), black_box(&ops)).unwrap())
    });

    c.bench_function("root", |b| {
        let vmval: [VmValue<InMemoryDB>; 2] = [empty_mtree.clone(), empty_mtree.clone()];
        let ops = ops![idx [0u8, 0u8]; root];
        b.iter(|| run_program(black_box(&vmval), black_box(&ops)).unwrap())
    });

    c.bench_function("pop", |b| {
        let vmval: [VmValue<InMemoryDB>; 1] = [vmval!((4u64))];
        let ops = ops![pop];
        b.iter(|| run_program(black_box(&vmval), black_box(&ops)).unwrap())
    });

    c.bench_function("popeq", |b| {
        let vmval: [VmValue<InMemoryDB>; 1] = [vmval!((4u64))];
        let ops = ops![popeq()];
        b.iter(|| run_program(black_box(&vmval), black_box(&ops)).unwrap())
    });

    c.bench_function("popeqc", |b| {
        let vmval: [VmValue<InMemoryDB>; 1] = [vmval!((4u64))];
        let ops = ops![popeqc()];
        b.iter(|| run_program(black_box(&vmval), black_box(&ops)).unwrap())
    });

    c.bench_function("add", |b| {
        let vmval: [VmValue<InMemoryDB>; 2] = [vmval!((4u64)), vmval!((5u64))];
        let ops = ops![add];
        b.iter(|| run_program(black_box(&vmval), black_box(&ops)).unwrap())
    });

    c.bench_function("addi", |b| {
        let vmval: [VmValue<InMemoryDB>; 1] = [vmval!((4u64))];
        let ops = ops![addi(2)];
        b.iter(|| run_program(black_box(&vmval), black_box(&ops)).unwrap())
    });

    c.bench_function("sub", |b| {
        let vmval: [VmValue<InMemoryDB>; 2] = [vmval!((4u64)), vmval!((3u64))];
        let ops = ops![sub];
        b.iter(|| run_program(black_box(&vmval), black_box(&ops)).unwrap())
    });

    c.bench_function("subi", |b| {
        let vmval: [VmValue<InMemoryDB>; 1] = [vmval!((4u64))];
        let ops = ops![subi(2)];
        b.iter(|| run_program(black_box(&vmval), black_box(&ops)).unwrap())
    });

    c.bench_function("push", |b| {
        let vmval: [VmValue<InMemoryDB>; 1] = [vmval!([null])];
        let ops = ops![push(2u64)];
        b.iter(|| run_program(black_box(&vmval), black_box(&ops)).unwrap())
    });

    c.bench_function("pushs", |b| {
        let vmval: [VmValue<InMemoryDB>; 1] = [vmval!([null])];
        let ops = ops![pushs(2u64)];
        b.iter(|| run_program(black_box(&vmval), black_box(&ops)).unwrap())
    });

    c.bench_function("branch", |b| {
        let vmval: [VmValue<InMemoryDB>; 1] = [vmval!((2u64))];
        let ops = ops![branch(1); noop (0)];
        b.iter(|| run_program(black_box(&vmval), black_box(&ops)).unwrap())
    });

    c.bench_function("jmp", |b| {
        let vmval: [VmValue<InMemoryDB>; 1] = [vmval!([null])];
        let ops = ops![jmp(1); noop (0)];
        b.iter(|| run_program(black_box(&vmval), black_box(&ops)).unwrap())
    });

    c.bench_function("concat", |b| {
        let vmval: [VmValue<InMemoryDB>; 2] = [vmval!((5u64)), vmval!((5u64))];
        let ops = ops![concat(4)];
        b.iter(|| run_program(black_box(&vmval), black_box(&ops)).unwrap())
    });

    c.bench_function("concatc", |b| {
        let vmval: [VmValue<InMemoryDB>; 2] = [vmval!((5u64)), vmval!((5u64))];
        let ops = ops![concatc(4)];
        b.iter(|| run_program(black_box(&vmval), black_box(&ops)).unwrap())
    });

    c.bench_function("member", |b| {
        let vmval: [VmValue<InMemoryDB>; 2] = [
            vmval!([{10u32 => null, 16u32 => null}]),
            vmval!([{10u32 => null, 16u32 => null}]),
        ];
        let ops = ops![idx [0u8]; push (16u32); member];
        b.iter(|| run_program(black_box(&vmval), black_box(&ops)).unwrap())
    });

    c.bench_function("rem", |b| {
        let vmval: [VmValue<InMemoryDB>; 1] = [vmval!([{10u32 => null, 16u32 => null}])];
        let ops = ops![idx [0u8]; push (16u32); rem];
        b.iter(|| run_program(black_box(&vmval), black_box(&ops)).unwrap())
    });

    c.bench_function("dup", |b| {
        let vmval: [VmValue<InMemoryDB>; 2] = [vmval!((5u64)), vmval!((5u64))];
        let ops = ops![dup(1)];
        b.iter(|| run_program(black_box(&vmval), black_box(&ops)).unwrap())
    });

    c.bench_function("swap", |b| {
        let vmval: [VmValue<InMemoryDB>; 3] = [vmval!((5u64)), vmval!((5u64)), vmval!((5u64))];
        let ops = ops![swap(1)];
        b.iter(|| run_program(black_box(&vmval), black_box(&ops)).unwrap())
    });

    c.bench_function("idx", |b| {
        let vmval: [VmValue<InMemoryDB>; 1] = [empty_mtree.clone()];
        let ops = ops![idx[0u8]];
        b.iter(|| run_program(black_box(&vmval), black_box(&ops)).unwrap())
    });

    c.bench_function("idxc", |b| {
        let vmval: [VmValue<InMemoryDB>; 1] = [vmval!(# [null])];
        let ops = ops![idxc[0u8]];
        b.iter(|| run_program(black_box(&vmval), black_box(&ops)).unwrap())
    });

    c.bench_function("ins", |b| {
        let vmval: [VmValue<InMemoryDB>; 3] = [
            vmval!((42u32)),
            vmval!((24u64)),
            vmval!([{42u32 => (24u64)}]),
        ];
        let ops = ops![idxp [0u8]; ins 1];
        b.iter(|| run_program(black_box(&vmval), black_box(&ops)).unwrap())
    });

    c.bench_function("ckpt", |b| {
        let vmval: [VmValue<InMemoryDB>; 1] = [vmval!([null])];
        let ops = ops![ckpt];
        b.iter(|| run_program(black_box(&vmval), black_box(&ops)).unwrap())
    });
}

criterion_group!(
    benchmarking,
    individual_vm_instructions,
    scaling_vm_instructions
);
criterion_main!(benchmarking);
