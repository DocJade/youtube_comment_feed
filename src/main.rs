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
use chrono::DateTime;
use curl::easy::{Easy, List};
use serde_json::Value;

// Set up command line arguments
#[derive(Parser, Debug)]
struct Args {
    // Youtube API token
    // Required.
    #[arg(long, required = true)]
    api_key: String,
}

// Store this bit of the youtube url to save space
const API_URL: &str = "https://youtube.googleapis.com/youtube/v3/";

fn main() {
    // Grab the Token from CLI
    let args = Args::parse();
    let api_key: &str = &args.api_key;

    // Test the token.
    println!("Testing API key...");
    match test_key(api_key) {
        Some(test_fail) => {
            match test_fail {
                TestFail::BadKey => println!("Bad API key!"),
                TestFail::CurlFailure(e) => println!("Curl failed! : {:?}", e),
                TestFail::SomethingBroke(e) => println!("Something broke! : {:?}", e),
            }
            std::process::exit(1)
        }
        None => (),
    }
    println!("API key is good!");
    println!("TEST lets grab some comments!");
    let comments: Vec<YTComment> = get_comments_from_video(api_key, "maxsZF-rsb4", 5).unwrap();
    println!("Comment 1: {}", comments[0].content)

    //TODO: get those comments!
}

#[derive(Debug)]
enum TestFail {
    CurlFailure(CurlFail),
    BadKey,
    SomethingBroke(String),
}

fn test_key(key: &str) -> Option<TestFail> {
    // Build the test URL
    let function: String = "search?part=snippet".to_string();
    let max_results: String = "&maxResults=1".to_string();
    let search_term: String = "&q=Never%20gonna%20give%20you%20up".to_string();
    let the_key: String = format!("&key={}", &key);
    let query: String = API_URL.to_owned() + &function + &max_results + &search_term + &the_key;
    // Run the query
    let mut result: std::result::Result<String, CurlFail> = c_get(&query);
    result = match result {
        Err(e) => return Some(TestFail::CurlFailure(e)),
        Ok(s) => Ok(s),
    };
    // Cool! We got some JSON! let's unwrap it and check it
    let json: Value = serde_json::from_str(&result.unwrap()).unwrap(); //TODO: double unwrap! ugly!
                                                                       // Now test if we got an error response.
    match json["error"]["code"].as_i64() {
        None => return None,                        // No error means test passed!
        Some(400) => return Some(TestFail::BadKey), // Token is no good!
        Some(_) => {
            return Some(TestFail::SomethingBroke(format!(
                "Failure checking token! {}",
                json
            )))
        } //number other than 400!
    }
}

// Make some easier functions for Curl

#[derive(Debug)]
enum CurlFail {
    SomethingBroke(String),
    BadURL,
    DataIssue,
    HeaderIssue,
}

fn c_get(input: &str) -> std::result::Result<String, CurlFail> {
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

#[derive(Debug)]
struct YTComment {
    content: String,
    author_name: String,
    timestamp: u64,
}

#[derive(Debug)]
enum CommentFail {
    NoComments,
    BadKey,
    CurlFailure(CurlFail),
    SomethingElse(String),
    EpochFail,
}

fn get_comments_from_video(
    key: &str,
    video_id: &str,
    amount: i8,
) -> Result<Vec<YTComment>, CommentFail> {
    //TODO: Filter out comments from self
    //https://www.googleapis.com/youtube/v3/commentThreads?key=[KEY]&textFormat=plainText&part=snippet&videoId=[VIDEO_ID]&maxResults=[AMOUNT]]

    // Create the Curl address.
    let _rq_type = "commentThreads?";
    let _key = format!("key={}&", key);
    let _format = "textFormat=plainText&";
    let _part = "part=snippet&";
    let _vid_id = format!("videoId={}&", video_id);
    let _num_results = format!("maxResults={}", amount);
    let _fields = "&fields=items(snippet(topLevelComment(snippet(authorDisplayName%2CtextOriginal%2CpublishedAt))))";
    let _url = format!(
        "{}{}{}{}{}{}{}{}",
        API_URL, _rq_type, _key, _format, _part, _vid_id, _num_results, _fields
    );

    // Run the query
    let _result: Result<String, CurlFail> = c_get(&_url);
    let mut _json: String;
    // Roll up errors
    match _result {
        Ok(okay) => _json = okay,
        Err(error) => return Err(CommentFail::CurlFailure(error)),
    }

    // We've got good JSON, time to pull the comments out of it.
    let unwrapped_json: Value = serde_json::from_str(&_json).unwrap();

    // First we need to check if we were given an error code.

    match unwrapped_json["error"]["code"].as_i64() {
        None => (),                                   // No error means test passed!
        Some(400) => return Err(CommentFail::BadKey), // Token is no good!
        Some(code) => {
            return Err(CommentFail::SomethingElse(format!(
                "Unknown response code! : {}",
                code
            )))
        }
    };

    // Okay, now that we know we have a good comment pull, lets scrape those comments out!

    // Json structure is as follows:
    //{
    //"items": [
    //  {
    //    "snippet": {
    //      "topLevelComment": {
    //        "snippet": {
    //          "textOriginal": "TEXT",
    //          "authorDisplayName": "NAME",
    //          "publishedAt": "TIME"
    //        }
    //      }
    //    }
    //  },

    let mut return_vec: Vec<YTComment> = Vec::new();

    let items_array = unwrapped_json["items"].as_array().unwrap();

    for item in items_array {
        let snippet = &item["snippet"];
        let top_level_comment = &snippet["topLevelComment"]["snippet"];

        let wrapped: YTComment = YTComment {
            content: top_level_comment["textOriginal"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
            author_name: top_level_comment["authorDisplayName"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
            timestamp: {
                let time_string = top_level_comment["publishedAt"].as_str().unwrap();
                let timestamp = DateTime::parse_from_rfc3339(time_string).unwrap();
                timestamp.timestamp().try_into().unwrap()
            },
        };

        // push that comment!
        return_vec.push(wrapped);
    }

    // Now check to make sure we got some comments before returning
    if return_vec.is_empty() {
        // No comments!
        return Err(CommentFail::NoComments);
    }
    return Ok(return_vec);
}

//fn c_post(input: String) -> Result<String, CurlFail> {
//    todo!()
//}
