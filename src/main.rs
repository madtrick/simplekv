mod command_log;
mod kv;
mod ping;
mod webserver;

use clap::Parser;
use kv::{start_kv_server, StartKVServerOptions};
use repl::{start_repl, StartReplOptions};
use std::time::Duration;
use std::{str::FromStr, thread};
use webserver::{start_webserver, StartWebserverOptions};

mod repl;

#[derive(Parser)]
struct Args {
    // Id of the KV node
    #[arg(long)]
    id: String,

    #[arg(long)]
    port: String,

    #[arg(long)]
    leader: bool,
}

fn main() {
    let args = Args::parse();
    let node_id = args.id;
    let port = args.port;
    // Cloned to avoid ownership issues between the threads which depend on the port value
    // TODO: what would have been a better way 👆🏻?
    let kv_port = port.clone();
    let kv_port_repl = port.clone();

    thread::spawn(move || {
        start_kv_server(StartKVServerOptions {
            node_id,
            port,
            peers: vec![String::from_str("localhost:1339").unwrap()],
            is_leader: args.leader,
        })
    });
    // Give some time for the server to start so that the repl and the webserver can open a
    // connection
    thread::sleep(Duration::new(1, 0));

    thread::spawn(move || start_webserver(StartWebserverOptions { kv_port }));
    thread::spawn(move || {
        start_repl(StartReplOptions {
            kv_port: kv_port_repl,
        })
    })
    .join()
    .unwrap();
}
