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

fn is_set_command(input: &str, regex: &Regex) -> Option<(String, String)> {
    regex
        .captures(input)
        .map(|capture| (capture[1].to_owned(), capture[2].to_owned()))
}

fn is_get_command(input: &str, regex: &Regex) -> Option<String> {
    regex.captures(input).map(|capture| capture[1].to_owned())
}
fn is_del_command(input: &str, regex: &Regex) -> Option<String> {
    regex.captures(input).map(|capture| capture[1].to_owned())
}

pub fn start_kv_server(node_id: &str) {
    // TODO read port from config
    let listener = TcpListener::bind("localhost:1337").unwrap();
    let get_regex = Arc::new(Regex::new(r"GET (\w+)").unwrap());
    let set_regex = Arc::new(Regex::new(r"SET (\w+) (\w+)").unwrap());
    let delete_regex = Arc::new(Regex::new(r"DEL (\w+)").unwrap());
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
        let get_regex = get_regex.clone();
        let delete_regex = delete_regex.clone();
        let set_regex = set_regex.clone();

        thread::spawn(move || {
            let stream = RefCell::new(stream);
            let stream_ref = &stream;
            // TODO understand why "stream_ref.borrow_mut" fails but "stream_ref.borrow_mut.try_clone"
            // works
            let buf_reader = BufReader::new(stream_ref.borrow_mut().try_clone().unwrap());

            // TODO there should be only one line. No need to iterate.
            for line in buf_reader.lines() {
                let request_line_string = line.unwrap();

                if let Some((key, value)) = is_set_command(&request_line_string, &set_regex) {
                    println!("KV server: SET {} = {}", key, value);
                    command_log.write().unwrap().append(Command::Set {
                        key: key.clone(),
                        value: value.clone(),
                    });
                    kv.write().unwrap().set(key, value)
                } else if let Some(key) = is_get_command(&request_line_string, &get_regex) {
                    println!("KV server: GET {}", key);

                    match kv.read().unwrap().get(&key) {
                        Some(value) => stream_ref
                            .borrow_mut()
                            .write_all(format!("{value}\n").as_bytes())
                            .unwrap(),
                        None => stream_ref.borrow_mut().write_all(b"__none__\n").unwrap(),
                    }
                } else if let Some(key) = is_del_command(&request_line_string, &delete_regex) {
                    println!("KV server: DEL {}", key);
                    command_log
                        .write()
                        .unwrap()
                        .append(Command::Delete { key: key.clone() });
                    kv.write().unwrap().del(&key);
                } else {
                    println!("KV server: uknown command {}", request_line_string)
                }
            }
        });

        // handle.join();
    }
}
