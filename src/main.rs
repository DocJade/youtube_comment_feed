// DocJade 2023

// Make Clippy angry
#![warn(
    clippy::pedantic,
    clippy::nursery,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::correctness,
    clippy::style,
    clippy::perf
)]

// Import the CLI argument parser
use clap::Parser;
// curl library
use curl::easy::Easy;

// Set up command line arguments
#[derive(Parser, Debug)]
struct Args {
    // Youtube API token
    // Required.
    #[arg(short, long, required = true)]
    api_token: String,
}

fn main() {
    // Grab the Token from CLI
    let args = Args::parse();
    let api_token = args.api_token;

    // Test the token.
    print!("{:?}", c_get(api_token.to_string()));
    //token_test = todo!()
}

// Make some easier functions for Curl

#[derive(Debug)]
enum CurlFail {
    AllGood,
    SomethingBroke,
    BadURL,
    DataIssue,
}

fn c_get(input: String) -> Result<String, CurlFail> {
    use std::sync::{Arc, Mutex};
    // create an easy from CURL
    let mut curl = Easy::new();
    // Arc and Mutex for shared mutability, lets us use the data in the closure.
    let data = Arc::new(Mutex::new(Vec::new()));

    // Set the URL

    match curl.url(&input) {
        Ok(_) => (),
        Err(_) => return Err(CurlFail::BadURL),
    }

    // Clone the Arc for the closure.
    let cloned_data = Arc::clone(&data);

    // Set a closure to write data to our Vec<u8>.
    let tmp = curl.write_function(move |response_data: &[u8]| {
        cloned_data.lock().unwrap().extend_from_slice(response_data);
        Ok(response_data.len())
    });

    match tmp {
        Ok(_) => (),
        Err(_) => return Err(CurlFail::DataIssue),
    }

    match curl.perform() {
        Ok(_) => (),
        Err(_) => return Err(CurlFail::SomethingBroke),
    }

    // Convert the Vec<u8> to a String.
    let response_string: String = String::from_utf8_lossy(&data.lock().unwrap()).to_string();
    Ok(response_string)
}

fn c_post(input: String) -> Result<String, CurlFail> {
    todo!()
}
