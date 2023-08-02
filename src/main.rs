use easy_repl::{command, CommandStatus, Repl};
use regex::Regex;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::{cell::RefCell, collections::HashMap, fs::File};

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

enum Command<'a> {
    Set { key: &'a str, value: &'a str },
    Delete { key: &'a str },
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
        }
    }
}

fn print_logfile(filename: &str) {
    // TODO: I wanted to do `File::open(filename)?` but it won't compile. Why?
    let file = File::open(filename).unwrap();
    let lines = BufReader::new(file).lines();
    let regex = Regex::new(r"(\w+)=(\w+)").unwrap();

    lines.for_each(|line| {
        if let Ok(line) = line {
            let Some(capture) = regex.captures(line.as_str()) else { return };

            println!("Key {}, Value {}", &capture[1], &capture[2])
        }
    })
}

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

fn launch_webserver() {
    let listener = TcpListener::bind("localhost:3333").unwrap();

    for stream in listener.incoming() {
        let mut stream = stream.unwrap();

        let buf_reader = BufReader::new(&mut stream);
        let http_request: Vec<_> = buf_reader
            .lines()
            .map(|result| result.unwrap())
            .take_while(|line| !line.is_empty())
            .collect();

        println!("Request: {:#?}", http_request);
    }
}

fn main() {
    let map = init_from_logfile("log");
    let log = RefCell::new(CommandLog::new("log"));
    let log_ref = &log;
    let kv = RefCell::new(KV::new(map));
    let kv_ref = &kv;

    // launch_webserver();

    // print_logfile("log");

    let mut repl = Repl::builder()
        .add(
            "SET",
            command! {
                "Set a value",
                (key: String, value: String) => |key: String, value: String| {
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
                    log_ref.borrow_mut().append(Command::Set { key: key.as_str(), value: value.as_str() });
                    Ok(CommandStatus::Done)
                }
            },
        )
        .add(
            "GET",
            command! {
                "Get a value",
                // TODO: replace &key with key
                (key: String) => |key| {
                    let kv = kv_ref.borrow();
                    let value = kv.get(&key);
                    match value {
                        Some(expr) => println!("GET {} = {}", key, expr),
                        None => println!("GET __none__"),
                    }


                    Ok(CommandStatus::Done)
                }
            },
        )
        .add(
            "DEL",
            command! {
                "Delete a value",
                (key: String) => |key: String| {
                    kv_ref.borrow_mut().del(key.clone());
                    log_ref.borrow_mut().append(Command::Delete { key: key.as_str() });

                    Ok(CommandStatus::Done)
                }
            },
        )
        .build()
        .expect("Failed to create repl");

    repl.run().expect("Critical REPL error");
}
