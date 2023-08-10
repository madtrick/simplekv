use regex::Regex;
use std::cell::RefCell;
use std::io::{BufRead, BufReader, Write};
use std::sync::{Arc, RwLock};
use std::thread;
use std::{collections::HashMap, net::TcpListener};

use crate::command_log::{self, CommandLog};
use crate::Command;

pub(crate) struct KV {
    map: HashMap<String, String>,
}

impl KV {
    pub fn new(map: HashMap<String, String>) -> KV {
        KV { map }
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

pub fn start_kv_server(node_id: &str) {
    // TODO read port from config
    let listener = TcpListener::bind("localhost:1337").unwrap();
    let kv = Arc::new(RwLock::new(KV::new(HashMap::new())));
    let command_log = Arc::new(RwLock::new(CommandLog::new(&format!("log.{node_id}"))));
    // let kv_ref = &kv;

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
