use regex::Regex;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::thread;
use std::{cell::RefCell, collections::HashMap, fs::File};

mod repl;

// TODO try pub(crate)
type MessageChannel = (
    Sender<(Message, ChannelEnd)>,
    Receiver<(Message, ChannelEnd)>,
);

struct KV {
    map: HashMap<String, String>,
}

impl KV {
    fn new(map: HashMap<String, String>) -> KV {
        KV { map }
    }

    fn set(&mut self, key: String, value: String) {
        self.map.insert(key, value);
    }

    fn get(&self, key: &String) -> Option<&String> {
        self.map.get(key)
    }

    fn del(&mut self, key: String) {
        self.map.remove(&key);
    }
}

#[derive(Debug)]
enum Command {
    Set { key: String, value: String },
    Delete { key: String },
    Get { key: String },
}

enum Message {
    Request(Command),
    Response(String),
}

struct CommandLog {
    file: File, // backing file to store the commands
}

fn __upsert_logfile(filename: &str) -> File {
    File::options()
        .append(true)
        .create(true)
        .open(filename)
        .unwrap()
}

impl CommandLog {
    fn new(filename: &str) -> CommandLog {
        CommandLog {
            file: __upsert_logfile(filename),
        }
    }

    fn append(&mut self, command: Command) {
        match command {
            Command::Set { key, value } => {
                println!("SET KEY {}, VALUE {}", key, value);
                writeln!(&mut self.file, "{}={}", key, value).unwrap();
            }
            Command::Delete { key } => {
                println!("DELETE KEY {}", key);
                writeln!(&mut self.file, "DEL {}", key).unwrap();
            }
            _ => panic!("Can't log this command"),
        }
    }
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
    let file = File::open(filename).unwrap();
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

fn launch_webserver(tx: Sender<(Message, ChannelEnd)>, rx: Receiver<(Message, ChannelEnd)>) {
    let listener = TcpListener::bind("localhost:3333").unwrap();

    for stream in listener.incoming() {
        let mut stream = stream.unwrap();

        let buf_reader = BufReader::new(&mut stream);
        let request_line_string = buf_reader.lines().next().unwrap().unwrap();
        let request_line = request_line_string.as_str();
        let get_regex = Regex::new(r"GET /get/(\w+) .+").unwrap();
        let set_regex = Regex::new(r"GET /set/(\w+)/(\w+) .+").unwrap();
        let delete_regex = Regex::new(r"GET /del/(\w+) .+").unwrap();

        let contents: String = if let Some(capture) = set_regex.captures(request_line) {
            println!("Key {}, Value {}", &capture[1], &capture[2]);
            tx.send((
                Message::Request(Command::Set {
                    key: capture[1].to_string(),
                    value: capture[2].to_string(),
                }),
                ChannelEnd::WebServer,
            ))
            .unwrap();
            "SET".to_string()
        } else if let Some(capture) = get_regex.captures(request_line) {
            println!("Get Key {}", &capture[1]);
            tx.send((
                Message::Request(Command::Get {
                    key: capture[1].to_string(),
                }),
                ChannelEnd::WebServer,
            ))
            .unwrap();
            let ( Message::Response(recv_message), _ ) = rx.recv().unwrap() else { panic!() };
            recv_message
        } else if let Some(capture) = delete_regex.captures(request_line) {
            println!("Del Key {}", &capture[1]);
            tx.send((
                Message::Request(Command::Delete {
                    key: capture[1].to_string(),
                }),
                ChannelEnd::WebServer,
            ))
            .unwrap();
            "DEL".to_string()
        } else {
            "UNKNOWN".to_string()
        };

        let contents_length = contents.len();
        let response =
            format!("HTTP/1.1 200 OK\r\nContent-Length: {contents_length}\r\n\r\n{contents}");

        println!("{}", response);
        stream.write_all(response.as_bytes()).unwrap();
    }
}

#[derive(PartialEq, Eq, Hash)]
enum ChannelEnd {
    WebServer,
    Repl,
    KV,
}

struct Dispatcher {
    map: HashMap<ChannelEnd, Sender<(Message, ChannelEnd)>>,
}

impl Dispatcher {
    fn new() -> Self {
        Dispatcher {
            map: HashMap::new(),
        }
    }

    fn register(&mut self, end: ChannelEnd, sender: Sender<(Message, ChannelEnd)>) {
        self.map.insert(end, sender);
    }

    fn dispatch(&mut self, from: ChannelEnd, to: ChannelEnd, message: Message) {
        let sender = self.map.get(&to).unwrap();
        sender.send((message, from)).unwrap();
    }
}

fn main() {
    let (tx_repl, rx_repl): MessageChannel = mpsc::channel();
    let (tx_server, rx_server): MessageChannel = mpsc::channel();
    let (tx_kv, rx_kv): MessageChannel = mpsc::channel();

    let tx_kv_webserver = tx_kv.clone();

    let repl_thread = thread::spawn(move || crate::repl::start_repl(tx_kv, rx_repl));
    thread::spawn(move || launch_webserver(tx_kv_webserver, rx_server));
    thread::spawn(move || {
        let map = init_from_logfile("log");
        let log = RefCell::new(CommandLog::new("log"));
        let log_ref = &log;
        let kv = RefCell::new(KV::new(map));
        let kv_ref = &kv;
        let mut dispatcher = Dispatcher::new();

        dispatcher.register(ChannelEnd::Repl, tx_repl.clone());
        dispatcher.register(ChannelEnd::WebServer, tx_server.clone());

        for (message, channel) in rx_kv.iter() {
            match message {
                Message::Request(Command::Set { key, value }) => {
                    println!("Set {} = {}", key, value);
                    /*
                     * I have to clone the key and value because the map inside the KV takes
                     * ownership of the key and value.
                     *
                     * See this response https://stackoverflow.com/a/32403439
                     *
                     * Though it might be better from a design point of view to pass a reference
                     * to the KV and let it do the cloning.
                     */
                    kv_ref.borrow_mut().set(key.clone(), value.clone());
                    log_ref.borrow_mut().append(Command::Set {
                        key: key.clone(),
                        value: value.clone(),
                    });

                    dispatcher.dispatch(ChannelEnd::KV, channel, Message::Response(value.clone()));
                }
                Message::Request(Command::Get { key }) => {
                    let kv = kv_ref.borrow();
                    let value = kv.get(&key);
                    let value = match value {
                        Some(expr) => {
                            println!("GET {} = {}", key, expr);
                            expr
                        }
                        None => {
                            println!("GET __none__");
                            "__none__"
                        }
                    };

                    dispatcher.dispatch(
                        ChannelEnd::KV,
                        channel,
                        Message::Response(value.to_string()),
                    );
                }
                Message::Request(Command::Delete { key }) => {
                    kv_ref.borrow_mut().del(key.clone());
                    log_ref.borrow_mut().append(Command::Delete { key });
                }
                _ => panic!("Unexpected message"),
            }
        }
    });

    // Don't exit until the repl thread exits
    repl_thread.join().unwrap();
}
