use regex::Regex;
use std::cell::RefCell;
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Write};
use std::sync::{Arc, RwLock};
use std::thread;
use std::{collections::HashMap, net::TcpListener};

use crate::command_log::CommandLog;
use crate::Command;

pub(crate) struct KV {
    map: HashMap<String, String>,
}

impl KV {
    pub fn new(map: HashMap<String, String>) -> KV {
        KV { map }
    }

    pub fn init_from_logfile(filename: &String) -> KV {
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

pub struct StartKVServerOptions {
    pub node_id: String,
    pub port: String,
}

pub fn start_kv_server(options: StartKVServerOptions) {
    let port = options.port;
    let node_id = options.node_id;
    let listener = TcpListener::bind(format!("localhost:{port}")).unwrap();
    let command_log = Arc::new(RwLock::new(CommandLog::new(format!("log.{node_id}"))));
    let kv = Arc::new(RwLock::new(KV::init_from_logfile(
        command_log.read().unwrap().filename(),
    )));

    for stream in listener.incoming() {
        println!("KV server: Accepted connection");
        let stream = stream.unwrap();
        /*
         * Clone the Arc to increase the refererence count and also
         * let the thread closure move this clone
         */
        let kv = kv.clone();
        let command_log = command_log.clone();

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
                let command = serde_json::from_str::<Command>(&request_line_string).unwrap();
                println!("{:?}", command);

                match command {
                    Command::Set { key, value } => {
                        println!("KV server: SET {} = {}", key, value);
                        command_log.write().unwrap().append(Command::Set {
                            key: key.clone(),
                            value: value.clone(),
                        });
                        kv.write().unwrap().set(key, value)
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
                    Command::Delete { key } => {
                        println!("KV server: DEL {}", key);
                        command_log
                            .write()
                            .unwrap()
                            .append(Command::Delete { key: key.clone() });
                        kv.write().unwrap().del(&key);
                    }
                }
            }
        });

        // handle.join();
    }
}
