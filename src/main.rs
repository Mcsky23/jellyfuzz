mod runner;
mod profiles;
mod parsing;
mod mutators;
mod utils;

use std::fs;
use tokio::fs::File as TokioFile;
use tokio::io::AsyncWriteExt;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::path::PathBuf;
use tokio::task::JoinHandle;
use clap::Parser;

use crate::mutators::literals::NumericTweaker;
use crate::parsing::parser::{generate_js, parse_js};
use crate::runner::pool::JobResult;
use crate::mutators::AstMutator;
use crate::mutators::minifier::Minifier;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    output_dir: PathBuf,
    // overwrite existing corpus files and start from scratch
    #[arg(short, long, action=clap::ArgAction::SetTrue)]
    overwrite: Option<bool>,
    // resume from existing corpus
    #[arg(short, long, action)]
    resume: Option<bool>,
    // the profile to use
    #[arg(short, long)]
    profile: Option<String>,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let output_dir = args.output_dir;
    let mutators = vec![
        Arc::new(NumericTweaker),
    ];

    if args.overwrite.unwrap_or(false) {
        // check if the directory exists
        if output_dir.exists() {
            // prompt user to confirm deletion
            println!("Output directory {:?} already exists. Are you sure you want to overwrite it? (y/N)", output_dir);
            let mut input = String::new();
            std::io::stdin().read_line(&mut input).unwrap();
            if input.trim().to_lowercase() == "y" {
                fs::remove_dir_all(&output_dir).unwrap();
                fs::create_dir_all(&output_dir).unwrap();
                println!("Overwritten existing directory.");
            } else {
                println!("Aborting.");
                return;
            }
        } else {
            fs::create_dir_all(&output_dir).unwrap();
        }

        // read initial corpus, minify and then check if the engine 
        // can execute it without syntax errors
    }


    // let src: Vec<u8> = fs::read("test.js").unwrap();
    // let script = parse_js(String::from_utf8(src).unwrap());
    // let new_script = NumericTweaker::mutate(script.unwrap());
    // // println!("Mutated Script: {:#?}", new_script.unwrap());
    // // println!("Mutated Script: {:#?}", new_script.unwrap());
    // let new_code = generate_js(new_script.unwrap());
    // fs::write("test_out.js", new_code.unwrap()).unwrap();
    // // let duration = start_time.elapsed();
    // // println!("Executed {} jobs in {:?}", cnt, duration);
    // // println!("{} errors", err);

}
