// DocJade 2023

// Import the CLI argument parser
use clap::Parser;

// Set up command line arguments
#[derive(Parser, Debug)]
struct Args {
    // Youtube API token
    // Required.
    #[arg(short, long, required = true)]
    api_token: String,
}

fn main() {
    let args = Args::parse();
    let api_token = args.api_token;
}
