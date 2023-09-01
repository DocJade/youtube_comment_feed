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
use curl::easy::{Easy, List};

// Set up command line arguments
#[derive(Parser, Debug)]
struct Args {
    // Youtube API token
    // Required.
    #[arg(long, required = true)]
    api_key: String
}

// Store this bit of the youtube url to save space
const API_URL: &str = "https://youtube.googleapis.com/youtube/v3/";

fn main() {
    // Grab the Token from CLI
    let args = Args::parse();
    let api_key: &str = &args.api_key;

    // Test the token.
    match test_token(api_key) {
        Some(test_fail) => panic!("Bad key or token: {:?}", test_fail),
        None => ()
    }

    //token_test = todo!()
}

#[derive(Debug)]
enum TestFail {
    CurlFailure(CurlFail),
    BadToken
}

fn test_token(key: &str) -> Option<TestFail> {
    // Build the test URL
    let function: String = "search?part=snippet".to_string();
    let max_results: String = "&maxResults=1".to_string();
    let search_term: String = "&q=Never%20gonna%20give%20you%20up".to_string();
    let the_key: String = format!("&key={}",&key);
    let query: String = API_URL.to_owned() + &function + &max_results + &search_term + &the_key;
    let result = c_get(&query);
    match result {
        Ok(_) => (),
        Err(e) => return Some(TestFail::CurlFailure(e)),
    };
    print!("{:?}",result);
    panic!("test")
}

// Make some easier functions for Curl

#[derive(Debug)]
enum CurlFail {
    SomethingBroke(String),
    BadURL,
    DataIssue,
    HeaderIssue
}

fn c_get(input: &str) -> Result<String, CurlFail> {
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

    // Set headers
    let mut headers = List::new();

    match headers.append("Accept: application/json") {
        Ok(_) => (),
        Err(_) => return Err(CurlFail::HeaderIssue),
    }
    
    match curl.http_headers(headers) {
        Ok(_) => (),
        Err(_) => return Err(CurlFail::HeaderIssue),
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
        Err(e) => return Err(CurlFail::SomethingBroke(e.to_string())),
    }

    // Convert the Vec<u8> to a String.
    let response_string: String = String::from_utf8_lossy(&data.lock().unwrap()).to_string();
    Ok(response_string)
}

//fn c_post(input: String) -> Result<String, CurlFail> {
//    todo!()
//}
