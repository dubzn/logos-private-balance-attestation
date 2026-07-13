#![no_main]

use risc0_zkvm::guest::env;

risc0_zkvm::guest::entry!(main);

fn main() {
    let value: u64 = env::read();
    let result = value.checked_add(1).expect("toy input overflow");
    env::commit(&result);
}

