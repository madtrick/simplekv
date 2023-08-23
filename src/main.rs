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
    follower: bool,
}

fn main() {
    let args = Args::parse();
    let node_id = args.id;
    let port = args.port;
    // Cloned to avoid ownership issues between the threads which depend on the port value
    // TODO: what would have been a better way 👆🏻?
    let kv_port = port.clone();
    let kv_port_repl = port.clone();

    /*
     * - If the node is not the leader then the kv server will take the ip and port of the leader
     * - The KV will register with the leader as a replica
     * - The leader will negotiate what information the replica is missing and send it
     * - It can start sending the full log
     * - The replica can request missing commands based on the sequence number
     */

    let leader = match args.follower {
        true => Some("localhost:1338".to_string()),
        false => None,
    };

    let handler = thread::spawn(move || {
        start_kv_server(StartKVServerOptions {
            node_id,
            port,
            leader,
        })
    });

    if !args.follower {
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

    handler.join().unwrap();
}
