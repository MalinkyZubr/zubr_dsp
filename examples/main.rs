mod basic;

use std::collections::HashMap;
use std::io;
use basic::audio_test::audio_test;
use basic::am_end_to_end::am_end_to_end_test;

fn main() {
    unsafe { backtrace_on_stack_overflow::enable() };
    let examples = HashMap::from(
        [
            ("audio_test",audio_test as fn() -> Result<(), String>),
            ("am_end_to_end", am_end_to_end_test as fn() -> Result<(), String>)
        ]
    );

    println!("Choose an example to run:");
    for (index, key) in examples.keys().enumerate() {
        println!("\t{}: {}", index, key);
    }

    println!("Enter your selection: ");
    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .expect("Failed to read line");

    let selection: usize = input.trim().parse().unwrap();
    let key = examples.keys().nth(selection).unwrap();

    match examples.get(key).unwrap()() {
        Ok(_) => println!("Example {} ran successfully", key),
        Err(e) => println!("Example {} failed with error: {}", key, e),
    }
}