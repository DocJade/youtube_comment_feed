// DocJade 2023

// Make Clippy angry
#![warn(
    clippy::pedantic,
    clippy::nursery,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::correctness,
    clippy::style,
    clippy::perf,
    clippy::complexity,
    clippy::cognitive_complexity,
    clippy::double_parens,
    clippy::len_zero,
    clippy::question_mark,
    clippy::suspicious,
    clippy::todo,
    //clippy::all  //for extra anger
)]

// TODO: convert emoji's into their names.

use core::panic;

// Import the CLI argument parser
use clap::Parser;
// curl library
use curl::easy::{Easy, List};
// json handling
use serde_json::Value;
// time handling
use chrono::DateTime;
// colored text
use colored::Colorize;

// Set up command line arguments
#[derive(Parser, Debug)]
struct Args {
    // Youtube API token
    // Required.
    #[arg(long, required = true)]
    api_key: String,
    #[arg(long, required = true)]
    channel_id: String,
}

// Store this bit of the youtube url to save space
const API_URL: &str = "https://youtube.googleapis.com/youtube/v3/";

#[derive(Debug, Clone)]
struct TrackedVideo {
    title: String,
    video_id: String,
    most_recent_timestamp: u64, // The timestamp of the most recent comment we saw last update.
    queued_comments: Vec<YTComment>, // Comments that are waiting for the print cycle.
    recheck_delay: u16,         // How many seconds to wait until next update.
}

fn main() {
    let args: Args = init();
    let api_key: &str = &args.api_key;
    let channel_id: &str = &args.channel_id;

    // Now that everything is ready to go, lets start tracking
    // comments!

    // The master Vec contains the TrackedVideo struct for... tracking videos.
    let mut master: Vec<TrackedVideo> = Vec::new();

    // Now we shall add all videos that currently exist on input channel
    // and set the most recent timestamp to NOW
    print!("Building tracked videos list... ");
    let returned = update_video_list(master.clone(), channel_id, api_key);
    match returned {
        Ok(okay) => master = okay,
        Err(error) => match error {
            ListUpdateError::ChannelIssue(e) => match e {
                ChannelVideosFail::NoVideos | ChannelVideosFail::BadKey => {
                    panic!("Should be unreachable! bad key or no videos!")
                }
                ChannelVideosFail::CurlFailure(e) => {
                    // we need to go deeper.
                    match e {
                        CurlFail::SomethingBroke(e) => {
                            print!("Unknown curl failure during first list build! : {e}");
                        }
                        CurlFail::BadURL | CurlFail::DataIssue | CurlFail::HeaderIssue => {
                            panic!("Should be unreachable! BadURL DataIssue HeaderIssue")
                        }
                    }
                }
                ChannelVideosFail::SomethingElse(e) => {
                    print!("Unknown failure during first list build! : {e}");
                    std::process::exit(1)
                }
            },
            ListUpdateError::SomethingElse(e) => {
                print!("Unknown failure during first list build! : {e}");
                std::process::exit(1)
            }
        },
    }
    println!("{}", "Done!".green());
    print!("Grabbing comments... ");
    master = match queue_comments(master, api_key) {
        Ok(okay) => okay,
        Err(error) => match error {
            CommentQueueFail::SomethingElse(error) => panic!("{error:?}"),
            CommentQueueFail::CommentFailed(_) => todo!(),
            CommentQueueFail::CurlFailed(_) => todo!(),
        },
    };
    println!("{}", "Done!".green());

    for video in master {
        println!("{}", format!("{}:\n", video.title).cyan());
        for comment in video.queued_comments {
            println!(
                "{}: {}\n",
                comment.author_name.to_string().blue(),
                comment.content
            );
        }
    }
}

fn init() -> Args {
    //setup and tests!

    // Grab the Token from CLI
    let args: Args = Args::parse();
    let api_key: &str = &args.api_key;
    let channel_id: &str = &args.channel_id;

    // Test the token.
    print!("Testing API key... ");
    test_key(api_key).map_or((), |test_fail| {
        match test_fail {
            KeyTestFail::BadKey => println!("Bad API key!"),
            KeyTestFail::CurlFailure(e) => println!("Curl failed! : {e:?}"),
            KeyTestFail::SomethingBroke(e) => println!("Something broke! : {e:?}"),
        }
        std::process::exit(1)
    });
    println!("{}", "API key is good!".green());
    print!("Testing Channel ID... ");

    match test_channel_id(channel_id, api_key) {
        Err(test_fail) => {
            match test_fail {
                ChannelTestFail::BadChannel => println!("Bad Channel ID!"),
                ChannelTestFail::CurlFailure(e) => println!("Curl failed! : {e:?}"),
                ChannelTestFail::SomethingBroke(e) => println!("Something broke! : {e:?}"),
            }
            std::process::exit(1)
        }
        Ok(name) => println!("{}", format!("Found {name:?}!").green()),
    }

    // Now get all video from the channel
    print!("Getting channel videos... ");

    let videos: Vec<Video>;

    match get_videos_from_channel(api_key, channel_id) {
        Ok(okay) => videos = okay,
        Err(fail) => {
            match fail {
                ChannelVideosFail::NoVideos => println!("Channel appears to have no videos!"),
                ChannelVideosFail::BadKey => println!("Key went bad?"),
                ChannelVideosFail::CurlFailure(e) => println!("Curl failed! : {e:?}"),
                ChannelVideosFail::SomethingElse(e) => println!("Something broke! : {e:?}"),
            }
            std::process::exit(1)
        }
    }
    println!(
        "{}",
        format!("Got {} channel videos!", videos.len()).green()
    );

    // Should have some videos now!
    // print one of them.

    println!(
        "Most recent video is {}.",
        format!("{:?}", videos[0].title).yellow()
    );

    // Now that we're done testing, return the args back to main.
    args
}

#[derive(Debug)]
enum KeyTestFail {
    CurlFailure(CurlFail),
    BadKey,
    SomethingBroke(String),
}

fn test_key(key: &str) -> Option<KeyTestFail> {
    // Build the test URL
    let function: String = "search?part=snippet".to_string();
    let max_results: String = "&maxResults=1".to_string();
    let search_term: String = "&q=Never%20gonna%20give%20you%20up".to_string();
    let the_key: String = format!("&key={}", &key);
    let query: String = API_URL.to_owned() + &function + &max_results + &search_term + &the_key;
    // Run the query
    let mut result: std::result::Result<String, CurlFail> = c_get(&query);
    result = match result {
        Err(e) => return Some(KeyTestFail::CurlFailure(e)),
        Ok(s) => Ok(s),
    };
    // Cool! We got some JSON! let's unwrap it and check it
    let json: Value = serde_json::from_str(&result.unwrap()).unwrap(); //TODO: double unwrap! ugly!
                                                                       // Now test if we got an error response.
    match json["error"]["code"].as_i64() {
        None => None,                           // No error means test passed!
        Some(400) => Some(KeyTestFail::BadKey), // Token is no good!
        Some(_) => Some(KeyTestFail::SomethingBroke(format!(
            "Failure checking token! {json}"
        ))), //number other than 400!
    }
}

enum ChannelTestFail {
    CurlFailure(CurlFail),
    BadChannel,
    SomethingBroke(String),
}

fn test_channel_id(channel_id: &str, key: &str) -> Result<String, ChannelTestFail> {
    // Is this channel real?
    // Build test URL:

    // Base URL
    let base_url = "channels";

    // Query parameters
    let part_param = "part=snippet";
    let id_param = format!("id={channel_id}");
    let fields_param = "fields=items(snippet(title))";

    // API Key
    let api_key = format!("&key={}", &key);

    // Combine the parts to create the full query
    let query = format!("{API_URL}{base_url}?{part_param}&{id_param}&{fields_param}&{api_key}");

    // Run the query
    let mut result: std::result::Result<String, CurlFail> = c_get(&query);

    // Make sure that curl went well.

    result = match result {
        Err(e) => return Err(ChannelTestFail::CurlFailure(e)),
        Ok(s) => Ok(s),
    };

    let json: Value = serde_json::from_str(&result.unwrap()).unwrap(); //TODO: double unwrap! ugly!

    // Check for error codes again

    match json["error"]["code"].as_i64() {
        None => (),                                           // No error means test passed!
        Some(400) => return Err(ChannelTestFail::BadChannel), // Token is no good!
        Some(_) => {
            return Err(ChannelTestFail::SomethingBroke(format!(
                "Failure checking channel! {json}"
            )))
        } //number other than 400!
    }

    //All good! return the channel name.

    let title = json["items"][0]["snippet"]["title"].as_str().unwrap();
    Ok(title.to_string())
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

    match curl.url(input) {
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

#[derive(Debug, Clone)]
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
    let rq_type = "commentThreads?";
    let key = format!("key={key}&");
    let format = "textFormat=plainText&";
    let part = "part=snippet&";
    let vid_id = format!("videoId={video_id}&");
    let num_results = format!("maxResults={amount}");
    let fields = "&fields=items(snippet(topLevelComment(snippet(authorDisplayName%2CtextOriginal%2CpublishedAt))))";
    let url = format!("{API_URL}{rq_type}{key}{format}{part}{vid_id}{num_results}{fields}");

    // Run the query
    let _result: Result<String, CurlFail> = c_get(&url);

    // Roll up errors
    let json: String = match _result {
        Ok(okay) => okay,
        Err(error) => return Err(CommentFail::CurlFailure(error)),
    };

    // We've got good JSON, time to pull the comments out of it.
    let unwrapped_json: Value = serde_json::from_str(&json).unwrap();

    // First we need to check if we were given an error code.

    match unwrapped_json["error"]["code"].as_i64() {
        None => (),                                   // No error means test passed!
        Some(400) => return Err(CommentFail::BadKey), // Token is no good!
        Some(code) => {
            panic!("Unknown response code! : {code} :: {unwrapped_json:?}")
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
    Ok(return_vec)
}

#[derive(Debug)]
enum ChannelVideosFail {
    NoVideos,
    BadKey,
    CurlFailure(CurlFail),
    SomethingElse(String),
}

#[derive(Debug)]
struct Video {
    title: String,
    id: String,
}

fn get_videos_from_channel(key: &str, channel_id: &str) -> Result<Vec<Video>, ChannelVideosFail> {
    // Lets get those videos
    // Create the URL for the API request
    let function = "search?part=snippet";
    let max_results = "&maxResults=4294967295";
    let order = "&order=date";
    let fields = "&fields=items(id(videoId)%2Csnippet(title))";
    let api_key = format!("&key={key}");
    let channel_param = format!("&channelId={channel_id}");
    let query = format!("{API_URL}{function}{max_results}{order}{fields}{api_key}{channel_param}");

    // run that query
    let result: Result<String, CurlFail> = c_get(&query);

    // Handle them errors.

    // Roll up errors
    let json: String = match result {
        Ok(okay) => okay,
        Err(error) => return Err(ChannelVideosFail::CurlFailure(error)),
    };

    // Good stuff, crack it open.
    let unwrapped_json: Value = serde_json::from_str(&json).unwrap();

    // Error handling again

    match unwrapped_json["error"]["code"].as_i64() {
        None => (),                                         // No error means test passed!
        Some(400) => return Err(ChannelVideosFail::BadKey), // Token is no good!
        Some(code) => {
            return Err(ChannelVideosFail::SomethingElse(format!(
                "Unknown response code! : {code}"
            )))
        }
    };

    // return format should match
    // {
    //     "items": [
    //       {
    //         "id": {
    //           "videoId": "ID"
    //         },
    //         "snippet": {
    //           "title": "TITLE"
    //         }
    //       },
    // }

    let mut return_vec: Vec<Video> = Vec::new();

    let items_array = unwrapped_json["items"].as_array().unwrap();
    let bad_chars = &['\"']; // Dont want these in our titles

    // Pull those titles and ID's out!

    for item in items_array {
        // check for nulls
        if item["snippet"]["title"] == Value::Null || item["id"]["videoId"] == Value::Null {
            // Nulls are a no-no, skip
            continue;
        }
        let wrapped: Video = Video {
            title: item["snippet"]["title"]
                .to_string()
                .trim()
                .replace(bad_chars, "")
                .to_string(),
            id: item["id"]["videoId"]
                .to_string()
                .trim()
                .replace(bad_chars, "")
                .to_string(),
        };
        // onto the vec it goes
        return_vec.push(wrapped);
    }

    Ok(return_vec)
}

#[derive(Debug)]
enum ListUpdateError {
    ChannelIssue(ChannelVideosFail),
    SomethingElse(String),
}

fn update_video_list(
    old: Vec<TrackedVideo>,
    channel_id: &str,
    key: &str,
) -> Result<Vec<TrackedVideo>, ListUpdateError> {
    // This function takes in the list of videos, checks the channel to see
    // if there are videos on the channel that do not exist in the list yet.

    // grab all of the videos off of the channel
    let current_videos = match get_videos_from_channel(key, channel_id) {
        Ok(okay) => okay,
        Err(error) => return Err(ListUpdateError::ChannelIssue(error)),
    };

    // Video list is good, now lets compare.

    // If the length of the 2 Vec are the same, its pretty safe to assume there hasnt been a new video.

    if old.len() == current_videos.len() {
        // The same! exit early
        return Ok(old);
    }

    // Build the new videos into a TrackedVideo
    let mut new_tracked_videos: Vec<TrackedVideo> = Vec::new();

    for i in current_videos {
        new_tracked_videos.push(TrackedVideo {
            title: i.title,
            video_id: i.id,
            most_recent_timestamp: 0,
            queued_comments: [].to_vec(),
            recheck_delay: 10,
        });
    }

    // Combine the new list with the old list
    // and remove dupes.

    //`old` comes first to make sure we discard matching news, not olds.
    let mut output: Vec<TrackedVideo> = old
        .iter()
        .chain(new_tracked_videos.iter())
        .cloned()
        .collect();
    output.sort_by(|a, b| a.video_id.cmp(&b.video_id));
    output.dedup_by(|a, b| a.video_id == b.video_id);

    // we're done!
    Ok(output)
}

enum CommentQueueFail {
    SomethingElse(String),
    CommentFailed(CommentFail),
    CurlFailed(CurlFail),
}

fn queue_comments(
    video_list: Vec<TrackedVideo>,
    key: &str,
) -> Result<Vec<TrackedVideo>, CommentQueueFail> {
    // This function takes in a list of tracked videos, and updates each entry with
    // new comments on those videos.

    let mut output_list: Vec<TrackedVideo> = Vec::new();

    // Loop over each video in the list!

    for video in video_list {
        // TODO: timed update checking.
        // if update not needed continue

        // Grab the most recent comments from this video
        // TODO: Increase comments retrieved until we find one with timestamp that older than video.most_recent_timestamp

        let comments: Vec<YTComment> = match get_comments_from_video(key, &video.video_id, 5) {
            Ok(messages) => messages,
            Err(error) => match error {
                CommentFail::NoComments => continue, // There are no comments, so there cant be any new ones either!
                CommentFail::BadKey | CommentFail::EpochFail => {
                    return Err(CommentQueueFail::CommentFailed(error))
                }
                CommentFail::CurlFailure(error) => return Err(CommentQueueFail::CurlFailed(error)),
                CommentFail::SomethingElse(error) => {
                    return Err(CommentQueueFail::SomethingElse(error))
                }
            },
        };

        // Now check if the comments are newer than most_recent_timestamp
        let mut new_comments: Vec<YTComment> = Vec::new();
        let mut out_updated: TrackedVideo = video.clone();
        let mut new_max_timestamp: u64 = video.most_recent_timestamp;

        for comment in comments {
            if comment.timestamp >= video.most_recent_timestamp {
                // comment is new! add to the list!
                // update the new timestamp if its the new best
                if comment.timestamp >= new_max_timestamp {
                    new_max_timestamp = comment.timestamp;
                }

                // add it to the comment buffer!
                new_comments.push(comment);
            }
        }

        // Add the comments (if there are any) to the TrackedVideo
        out_updated.queued_comments.append(&mut new_comments);

        // Apply the new most recent timestamp
        out_updated.most_recent_timestamp = new_max_timestamp;

        // Now push the updated video to the output
        output_list.push(out_updated);
    }

    // All the comments should be updated now!
    Ok(output_list)
}
