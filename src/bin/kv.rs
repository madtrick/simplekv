use clap::Parser;
use regex::Regex;
use rustkv::{Command, Connect, ConnectOk, Message, ReplicationCommand};
use std::cell::RefCell;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Result as IOResult, Write};
use std::net::TcpStream;
use std::sync::{Arc, RwLock};
use std::thread;
use std::vec;
use std::{collections::HashMap, net::TcpListener, time::Duration};

struct ReplicationPeer {
    pub peer: String,
    pub stream: TcpStream,
}

impl ReplicationPeer {
    fn new(peer: &String, stream: TcpStream) -> Self {
        ReplicationPeer {
            peer: peer.clone(),
            stream,
        }
    }

    fn replicate(&mut self, command: Command, sequence: usize) -> IOResult<()> {
        println!("Replicate ");
        let response = self.stream.write_all(
            format!(
                "{}\n",
                serde_json::to_string(&Message::ReplicationCommand(ReplicationCommand {
                    command,
                    sequence,
                }))
                .unwrap()
            )
            .as_bytes(),
        );

        response
    }
}
pub(crate) struct KV {
    pub map: HashMap<String, String>,
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

pub(crate) struct CommandLog {
    file: File,       // backing file to store the commands
    filename: String, // name of the file
    sequence: usize,
}

fn upsert_logfile(filename: &String) -> File {
    File::options()
        .append(true)
        .create(true)
        .open(filename)
        .unwrap()
}

fn count_lines(filename: &String) -> usize {
    // NOTE: I wanted to pass the `File` returned from `upsert_logfile` when I did so the
    // could would hang. The CommandLog code wouldn't execute pass the call to this
    // function.
    // TODO: read only the last line and get its sequence value
    BufReader::new(File::open(filename).unwrap())
        .lines()
        .count()
}

impl CommandLog {
    pub fn new(filename: String) -> CommandLog {
        let file = upsert_logfile(&filename);
        let sequence = count_lines(&filename);

        println!("Sequence {}", sequence);

        CommandLog {
            file,
            filename,
            sequence,
        }
    }

    pub fn append(&mut self, command: &Command) -> usize {
        match command {
            Command::Set { ref key, ref value } => {
                writeln!(&mut self.file, "{}#{}={}", self.sequence, key, value).unwrap();
            }
            Command::Delete { ref key } => {
                writeln!(&mut self.file, "{}#DEL {}", self.sequence, key).unwrap();
            }
            _ => panic!("Can't log this command"),
        }

        self.sequence += 1;
        self.sequence
    }

    pub fn replicated_append(&mut self, command: &Command, sequence: usize) -> usize {
        match command {
            Command::Set { ref key, ref value } => {
                writeln!(&mut self.file, "{}#{}={}", sequence, key, value).unwrap();
            }
            Command::Delete { ref key } => {
                writeln!(&mut self.file, "{}#DEL {}", sequence, key).unwrap();
            }
            _ => panic!("Can't log this command"),
        }

        sequence
    }

    pub fn filename(&self) -> &String {
        &self.filename
    }
}

fn handle_replica_stream(stream: TcpStream, kv: KV, command_log: CommandLog) {
    let stream = RefCell::new(stream);
    let stream_ref = &stream;
    let kv = &RefCell::new(kv);
    let command_log = &RefCell::new(command_log);

    let buf_reader = BufReader::new(stream_ref.borrow_mut().try_clone().unwrap());

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
            Message::ConnectOk(ConnectOk { map }) => {
                for (key, value) in map {
                    kv.borrow_mut().set(key, value)
                }
            }
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
                            .borrow_mut()
                            .replicated_append(&command, sequence);

                        println!("Sequence {}", sequence);
                        kv.borrow_mut().set(key.clone(), value.clone());
                    }
                    Command::Get { key: _ } => {
                        panic!("GET commands can't be replicated")
                    }
                    Command::Delete { ref key } => {
                        println!("KV server: DEL {}", key);
                        command_log
                            .borrow_mut()
                            .replicated_append(&command, sequence);
                        kv.borrow_mut().del(key);
                    }
                }
            }
            _ => panic!("Unhandled message"),
        }
    }

    // This is needed when the exiting thread is one handle a connection for a replication peer
    // if let Some(replication_peer) = replication_peer {
    //     replication_peers
    //         .write()
    //         .unwrap()
    //         .swap_remove(replication_peer);
    // }
}

fn handle_stream(
    stream: TcpStream,
    kv: Arc<RwLock<KV>>,
    command_log: Arc<RwLock<CommandLog>>,
    replication_peers: Arc<RwLock<Vec<ReplicationPeer>>>,
) {
    let stream = RefCell::new(stream);
    let stream_ref = &stream;
    // TODO understand why "stream_ref.borrow_mut" fails but "stream_ref.borrow_mut.try_clone"
    // works
    let buf_reader = BufReader::new(stream_ref.borrow_mut().try_clone().unwrap());
    let mut replication_peer: Option<usize> = None;

    for line in buf_reader.lines() {
        let request_line_string = line.unwrap();
        println!("MESSAGE {}", request_line_string);
        if request_line_string == "PING" {
            println!("PING request");
            stream_ref.borrow_mut().write_all(b"PONG\n").unwrap();
            continue;
        }

        let message = serde_json::from_str::<Message>(&request_line_string).unwrap();
        let mut dead_peers: Vec<usize> = Vec::new();
        println!("{:?}", message);

        match message {
            Message::Command(command) => {
                match command {
                    // TODO: this commands can't be handled by a replica because the
                    // repl is sending the commands to the wrong host (the leader) and because
                    // the replica is not listening to any other connection than the one opened
                    // with the leader
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
                        for (index, replication_peer) in replication_peers.iter_mut().enumerate() {
                            println!("Replicate to {}", replication_peer.peer);
                            match replication_peer.replicate(command.clone(), sequence) {
                                Ok(_) => (),
                                Err(e) => {
                                    println!("{}", e);
                                    dead_peers.push(index);
                                }
                            }
                        }
                    }
                    Command::Get { key } => {
                        println!("KV server: GET {}", key);

                        match kv.read().unwrap().get(&key) {
                            Some(value) => stream_ref
                                .borrow_mut()
                                .write_all(format!("{value}\n").as_bytes())
                                .unwrap(),
                            None => stream_ref.borrow_mut().write_all(b"__none__\n").unwrap(),
                        }
                    }
                    Command::Delete { ref key } => {
                        println!("KV server: DEL {}", key);
                        let sequence = command_log.write().unwrap().append(&command);
                        kv.write().unwrap().del(key);

                        let mut replication_peers = replication_peers.write().unwrap();
                        for (index, replication_peer) in replication_peers.iter_mut().enumerate() {
                            println!("Replicate to {}", replication_peer.peer);
                            match replication_peer.replicate(command.clone(), sequence) {
                                Ok(_) => (),
                                Err(e) => {
                                    println!("{}", e);
                                    dead_peers.push(index);
                                }
                            }
                        }
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
                        .push(ReplicationPeer::new(
                            &from,
                            stream_ref.borrow().try_clone().unwrap(),
                        ));

                    replication_peer = Some(replication_peers.read().unwrap().len() - 1);

                    stream_ref
                        .borrow_mut()
                        .write_all(
                            format!(
                                "{}\n",
                                serde_json::to_string(&Message::ConnectOk(ConnectOk {
                                    // TODO: do not read directly the map from the KV
                                    map: kv.read().unwrap().map.clone()
                                }))
                                .unwrap()
                            )
                            .as_bytes(),
                        )
                        .unwrap()
                }
                _ => panic!("Unexpected connect payload"),
            },
            _ => panic!("Unexpected message"),
        }

        // This is needed when the the stream opened for the REPL or webserver
        // handle an error after trying to write to a closed socket because the peer is gone.
        for dead_peer in dead_peers {
            replication_peers.write().unwrap().swap_remove(dead_peer);
        }
    }

    println!("ADIEU");

    // This is needed when the exiting thread is one handle a connection for a replication peer
    if let Some(replication_peer) = replication_peer {
        replication_peers
            .write()
            .unwrap()
            .swap_remove(replication_peer);
    }
}

#[derive(Parser)]
struct Args {
    #[arg(long)]
    port: String,

    // Id of the KV node
    #[arg(long)]
    id: String,
}

fn open_replica_stream(address: &str) -> TcpStream {
    loop {
        match TcpStream::connect(address) {
            Ok(stream) => return stream,
            Err(_) => thread::sleep(Duration::from_millis(4000)),
        }
    }
}

pub fn main() {
    let args = Args::parse();
    let node_id = args.id;
    let port = args.port.parse::<u16>().unwrap();
    let leader: Option<String> = None;
    println!("Listening on port {}", port);
    let listener = TcpListener::bind(format!("localhost:{port}")).unwrap();
    let command_log = Arc::new(RwLock::new(CommandLog::new(format!("log.{node_id}"))));
    let kv = Arc::new(RwLock::new(KV::init_from_logfile(
        command_log.read().unwrap().filename(),
    )));
    let replications = HashMap::from([
        (1338 as u16, vec!["1339", "1340"]),
        (1339 as u16, vec!["1338", "1340"]),
        (1340 as u16, vec!["1338", "1339"]),
    ]);
    let replication_peers: Vec<ReplicationPeer> = Vec::new();
    let replication_peers = Arc::new(RwLock::new(replication_peers));

    match replications.get(&port) {
        Some(replicas) => {
            for leader_port in replicas {
                let mut stream = open_replica_stream(format!("localhost:{}", leader_port).as_str());

                stream
                    .write_all(
                        format!(
                            "{}\n",
                            serde_json::to_string(&Message::Connect(Connect {
                                // TODO: do we need to send the address of the peer. The KV could get it
                                // from the connection
                                from: format!("localhost:{port}") // TODO: include current sequence number in the replica
                            }))
                            .unwrap()
                        )
                        .as_bytes(),
                    )
                    .unwrap();
                let replica_command_log = CommandLog::new(format!("log.{node_id}.{leader_port}"));
                let replica_kv = KV::init_from_logfile(command_log.read().unwrap().filename());

                thread::spawn(|| handle_replica_stream(stream, replica_kv, replica_command_log));
            }
        }
        None => panic!("Unknown port"),
    }

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
            handle_stream(
                stream.try_clone().unwrap(),
                kv,
                command_log,
                replication_peers,
            );
            println!("Thread exiting {}", stream.peer_addr().unwrap());
        });
    }

    // if options.is_leader {
    //     start_ping(StartPingOptions {
    //         peers: options.peers,
    //     });
    // }
    //

    // handle.join();
}
