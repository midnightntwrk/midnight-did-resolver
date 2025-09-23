use midnight_onchain_runtime::serialize::{Deserializable, Serializable};
use onchain_runtime_state::state::StateValue;
use onchain_vm::cost_model::DUMMY_COST_MODEL;
use onchain_vm::ops::Op;
use onchain_vm::result_mode::ResultModeGather;
use onchain_vm::storage::DefaultDB;
use onchain_vm::vm::run_program;
use onchain_vm::vm_value::{ValueStrength, VmValue};
use std::fs::File;
use std::{
    env::args,
    io::{BufRead, BufReader},
    process::exit,
};

fn main() {
    let args = args().collect::<Vec<_>>();
    if args.len() != 3 && args.len() != 4 {
        println!("Usage: {} <STATE-IN> <PROGRAM> [<STATE-OUT>]", args[0]);
    } else {
        let state_in: StateValue<DefaultDB> =
            <StateValue<DefaultDB> as Deserializable>::deserialize(
                &mut File::open(&args[1]).unwrap(),
                0,
            )
            .unwrap();
        println!("Input state: {:#?}", &state_in);
        let prog_in: Vec<Op<ResultModeGather, DefaultDB>> = {
            let mut prog = Vec::new();
            let mut file = BufReader::new(File::open(&args[2]).unwrap());
            while !file.fill_buf().unwrap().is_empty() {
                prog.push(Deserializable::deserialize(&mut file, 0).unwrap());
            }
            prog
        };
        println!("Input program: {:#?}", &prog_in);

        match run_program(
            &[VmValue::new(ValueStrength::Strong, state_in)],
            &prog_in[..],
            None,
            &DUMMY_COST_MODEL,
        ) {
            Ok(res) => {
                println!("Result stack: {:#?}", &res.stack);
                println!("Events output: {:#?}", &res.events);
                println!("Gas cost: {}", &res.gas_cost);

                if args.len() == 4 {
                    Serializable::serialize(
                        &res.stack[0].value,
                        &mut File::create(&args[3]).unwrap(),
                    )
                    .unwrap();
                }
            }
            Err(err) => {
                println!("Program failed with error: {err:?}");
                exit(1);
            }
        }
    }
}
