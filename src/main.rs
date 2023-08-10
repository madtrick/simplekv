mod command_log;
mod kv;
mod webserver;

use clap::Parser;
use kv::start_kv_server;
use regex::Regex;
use repl::start_repl;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader};
use std::thread;
use std::time::Duration;
use webserver::launch_webserver;

mod repl;

#[derive(Debug, Serialize, Deserialize)]
enum Command {
    Set { key: String, value: String },
    Delete { key: String },
    Get { key: String },
}

// fn print_logfile(filename: &str) {
//     // TODO: I wanted to do `File::open(filename)?` but it won't compile. Why?
//     let file = File::open(filename).unwrap();
//     let lines = BufReader::new(file).lines();
//     let regex = Regex::new(r"(\w+)=(\w+)").unwrap();
//
//     lines.for_each(|line| {
//         if let Ok(line) = line {
//             let Some(capture) = regex.captures(line.as_str()) else { return };
//
//             println!("Key {}, Value {}", &capture[1], &capture[2])
//         }
//     })
// }

fn init_from_logfile(filename: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(filename)
        .unwrap();
    let lines = BufReader::new(file).lines();
    let set_regex = Regex::new(r"(\w+)=(\w+)").unwrap();
    let del_regex = Regex::new(r"DEL (\w+)").unwrap();

    lines.for_each(|line| {
        if let Ok(line) = line {
            // This patter of `let x = foo else bar` is called let-else. https://rust-lang.github.io/rfcs/3137-let-else.html
            if let Some(capture) = set_regex.captures(line.as_str()) {
                println!("Key {}, Value {}", &capture[1], &capture[2]);
                map.insert(String::from(&capture[1]), String::from(&capture[2]));
            } else if let Some(capture) = del_regex.captures(line.as_str()) {
                println!("Del Key {}", &capture[1]);
                map.remove(&capture[1]);
            };
        }
    });

    map
}

#[derive(Parser)]
struct Args {
    // Id of the KV node
    #[arg(long)]
    id: String,
}

fn main() {
    let args = Args::parse();
    let node_id = args.id;

    thread::spawn(move || start_kv_server(&node_id));
    // Give some time for the server to start so that the repl and the webserver can open a
    // connection
    thread::sleep(Duration::new(1, 0));

    thread::spawn(launch_webserver);
    thread::spawn(start_repl).join().unwrap();
}
