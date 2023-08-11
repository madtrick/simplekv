mod command_log;
mod kv;
mod webserver;

use clap::Parser;
use kv::start_kv_server;
use repl::start_repl;
use serde::{Deserialize, Serialize};
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
