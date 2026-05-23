use std::io::{self, Write};

pub fn check_confirmation() -> bool {
    print!(">> yes or no ");

    io::stdout().flush().unwrap();

    let mut input = String::new();

    io::stdin().read_line(&mut input).expect("Failed to read");

    let input = input.trim();

    input == "yes"
}
