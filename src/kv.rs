use regex::Regex;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::sync::{Arc, RwLock};
use std::thread;
use std::{collections::HashMap, net::TcpListener};

use crate::command_log::CommandLog;
// use crate::ping::{start_ping, StartPingOptions};

pub(crate) struct KV {
    map: HashMap<String, String>,
}

impl KV {
    pub fn new(map: HashMap<String, String>) -> KV {
        KV { map }
    }

    // TODO: this method should be moved to the `CommandLog`
    pub fn init_from_logfile(filename: &String) -> KV {
        let mut map = HashMap::new();
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(filename)
            .unwrap();
        let lines = BufReader::new(file).lines();
        let set_regex = Regex::new(r"\d+#(\w+)=(\w+)").unwrap();
        let del_regex = Regex::new(r"\d+#DEL (\w+)").unwrap();

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

        KV::new(map)
    }

    pub fn set(&mut self, key: String, value: String) {
        self.map.insert(key, value);
    }

    pub fn get(&self, key: &String) -> Option<&String> {
        self.map.get(key)
    }

    // TODO take a &str instead
    pub fn del(&mut self, key: &String) {
        self.map.remove(key);
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Command {
    Set { key: String, value: String },
    Delete { key: String },
    Get { key: String },
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Connect {
    pub from: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Message {
    Command(Command),
    ReplicationCommand(ReplicationCommand),
    Connect(Connect),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReplicationCommand {
    pub command: Command,
    pub sequence: usize,
}

struct ReplicationPeer {
    pub peer: String,
    pub stream: TcpStream,
}

impl ReplicationPeer {
    fn new(peer: &String) -> Self {
        ReplicationPeer {
            peer: peer.clone(),
            stream: TcpStream::connect(peer).unwrap(),
        }
    }

    fn replicate(&mut self, command: Command, sequence: usize) {
        println!("Replicate ");
        self.stream
            .write_all(
                format!(
                    "{}\n",
                    serde_json::to_string(&Message::ReplicationCommand(ReplicationCommand {
                        command,
                        sequence,
                    }))
                    .unwrap()
                )
                .as_bytes(),
            )
            .unwrap();
    }
}

pub struct StartKVServerOptions {
    pub node_id: String,
    pub port: String,
    pub leader: Option<String>,
}

pub fn start_kv_server(options: StartKVServerOptions) {
    let port = options.port;
    let node_id = options.node_id;
    let listener = TcpListener::bind(format!("localhost:{port}")).unwrap();
    let command_log = Arc::new(RwLock::new(CommandLog::new(format!("log.{node_id}"))));
    let kv = Arc::new(RwLock::new(KV::init_from_logfile(
        command_log.read().unwrap().filename(),
    )));
    let replication_peers: Vec<ReplicationPeer> = Vec::new();
    let replication_peers = Arc::new(RwLock::new(replication_peers));

    if let Some(leader) = options.leader {
        let mut stream = TcpStream::connect(&leader).unwrap();

        stream
            .write_all(
                format!(
                    "{}\n",
                    serde_json::to_string(&Message::Connect(Connect {
                        from: format!("localhost:{port}")
                    }))
                    .unwrap()
                )
                .as_bytes(),
            )
            .unwrap();

        let mut reader = BufReader::new(stream.try_clone().unwrap());
        let mut value = String::new();
        reader.read_line(&mut value).unwrap();
    }

    // if options.is_leader {
    //     start_ping(StartPingOptions {
    //         peers: options.peers,
    //     });
    // }
    //
    for stream in listener.incoming() {
        let stream = stream.unwrap();
        println!(
            "KV server: Accepted connection {}",
            stream.peer_addr().unwrap()
        );
        /*
         * Clone the Arc to increase the refererence count and also
         * let the thread closure move this clone
         */
        let kv = kv.clone();
        let command_log = command_log.clone();
        let replication_peers = replication_peers.clone();

        thread::spawn(move || {
            let stream = RefCell::new(stream);
            let stream_ref = &stream;
            // TODO understand why "stream_ref.borrow_mut" fails but "stream_ref.borrow_mut.try_clone"
            // works
            let buf_reader = BufReader::new(stream_ref.borrow_mut().try_clone().unwrap());

            // TODO there should be only one line. No need to iterate.
            for line in buf_reader.lines() {
                let request_line_string = line.unwrap();
                println!("MESSAGE {}", request_line_string);
                if request_line_string == "PING" {
                    println!("PING request");
                    stream_ref.borrow_mut().write_all(b"PONG\n").unwrap();
                    continue;
                }

                let message = serde_json::from_str::<Message>(&request_line_string).unwrap();
                println!("{:?}", message);

                match message {
                    Message::Command(command) => {
                        match command {
                            Command::Set { ref key, ref value } => {
                                println!("KV server: SET {} = {}", key, value);
                                // let sequence = command_log.write().unwrap().append(Command::Set {
                                //     key: key.clone(),
                                //     value: value.clone(),
                                // });

                                let sequence = command_log.write().unwrap().append(&command);

                                println!("Sequence {}", sequence);
                                kv.write().unwrap().set(key.clone(), value.clone());
                                // stream_ref.borrow_mut().write_all(
                                //     serde_json::to_string(&ReplicationCommand { command, sequence })
                                //         .unwrap()
                                //         .as_bytes(),
                                // );
                                let mut replication_peers = replication_peers.write().unwrap();
                                for replication_peer in replication_peers.iter_mut() {
                                    println!("Replicate to {}", replication_peer.peer);
                                    replication_peer.replicate(command.clone(), sequence)
                                }
                            }
                            Command::Get { key } => {
                                println!("KV server: GET {}", key);

                                match kv.read().unwrap().get(&key) {
                                    Some(value) => stream_ref
                                        .borrow_mut()
                                        .write_all(format!("{value}\n").as_bytes())
                                        .unwrap(),
                                    None => {
                                        stream_ref.borrow_mut().write_all(b"__none__\n").unwrap()
                                    }
                                }
                            }
                            Command::Delete { ref key } => {
                                println!("KV server: DEL {}", key);
                                command_log.write().unwrap().append(&command);
                                kv.write().unwrap().del(key);
                            }
                        }
                    }
                    Message::Connect(data) => match data {
                        // TODO: can I do the matching on the line above instead of using another
                        // `match` expression
                        Connect { from } => {
                            println!("Connect from {}", from);
                            replication_peers
                                .write()
                                .unwrap()
                                .push(ReplicationPeer::new(&from))
                        }
                        _ => panic!("Unexpected connect payload"),
                    },
                    Message::ReplicationCommand(replication) => {
                        println!("Replication {:?}", replication);
                        let command = replication.command;
                        let sequence = replication.sequence;

                        match command {
                            Command::Set { ref key, ref value } => {
                                println!("KV server: SET {} = {}", key, value);
                                // let sequence = command_log.write().unwrap().append(Command::Set {
                                //     key: key.clone(),
                                //     value: value.clone(),
                                // });

                                let sequence = command_log
                                    .write()
                                    .unwrap()
                                    .replicated_append(&command, sequence);

                                println!("Sequence {}", sequence);
                                kv.write().unwrap().set(key.clone(), value.clone());
                                // stream_ref.borrow_mut().write_all(
                                //     serde_json::to_string(&ReplicationCommand { command, sequence })
                                //         .unwrap()
                                //         .as_bytes(),
                                // );
                                let mut replication_peers = replication_peers.write().unwrap();
                                for replication_peer in replication_peers.iter_mut() {
                                    replication_peer.replicate(command.clone(), sequence)
                                }
                            }
                            Command::Get { key } => {
                                println!("KV server: GET {}", key);

                                match kv.read().unwrap().get(&key) {
                                    Some(value) => stream_ref
                                        .borrow_mut()
                                        .write_all(format!("{value}\n").as_bytes())
                                        .unwrap(),
                                    None => {
                                        stream_ref.borrow_mut().write_all(b"__none__\n").unwrap()
                                    }
                                }
                            }
                            Command::Delete { ref key } => {
                                println!("KV server: DEL {}", key);
                                command_log
                                    .write()
                                    .unwrap()
                                    .replicated_append(&command, sequence);
                                kv.write().unwrap().del(key);
                            }
                        }
                    }
                }
            }
        });

        // handle.join();
    }
}
