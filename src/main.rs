use easy_repl::{command, CommandStatus, Repl};
use regex::Regex;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::{cell::RefCell, collections::HashMap, fs::File};

struct KV {
    map: HashMap<String, String>,
}

impl KV {
    fn new() -> KV {
        KV {
            map: HashMap::new(),
        }
    }

    fn set(&mut self, key: String, value: String) {
        // TODO: how can I avoid cloning the key and value here?
        self.map.insert(key, value);
    }

    fn get(&self, key: &String) -> Option<&String> {
        self.map.get(key)
    }

    fn del(&mut self, key: String) {
        self.map.remove(&key);
    }
}

fn upsert_logfile(filename: &str) -> File {
    File::options()
        .append(true)
        .create(true)
        .open(filename)
        .unwrap()
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
    // let map = init_from_logfile("log");
    // let values = RefCell::new(map);
    // let values_ref = &values;
    // let file = RefCell::new(upsert_logfile("log"));
    // let file_ref = &file;
    let kv = RefCell::new(KV::new());
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
                    kv_ref.borrow_mut().set(key, value);
                    // let mut file = file_ref.borrow_mut();

                    // TODO: how can I avoid cloning the key and value here?
                    // map.insert(key.clone(), value.clone());

                    // writeln!(file, "{}={}", key, value).unwrap();
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
                (key: String) => |key| {
                    kv_ref.borrow_mut().del(key);
                    // let mut file = file_ref.borrow_mut();

                    // writeln!(&mut file, "DEL {}", key).unwrap();

                    Ok(CommandStatus::Done)
                }
            },
        )
        .build()
        .expect("Failed to create repl");

    repl.run().expect("Critical REPL error");
}
