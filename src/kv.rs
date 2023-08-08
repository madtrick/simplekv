use regex::Regex;
use std::cell::RefCell;
use std::io::{BufRead, BufReader, Write};
use std::{collections::HashMap, net::TcpListener};

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

    pub fn del(&mut self, key: String) {
        self.map.remove(&key);
    }
}

fn is_set_command(input: &str, regex: &Regex) -> Option<(String, String)> {
    regex
        .captures(input)
        .map(|capture| (capture[1].to_owned(), capture[2].to_owned()))
}

fn is_get_command(input: &str, regex: &Regex) -> Option<String> {
    regex.captures(input).map(|capture| capture[0].to_owned())
}
fn is_del_command(input: &str, regex: &Regex) -> Option<String> {
    regex.captures(input).map(|capture| capture[0].to_owned())
}

pub fn start_kv_server() {
    let listener = TcpListener::bind("localhost:1337").unwrap();
    let get_regex = Regex::new(r"GET (\w+)").unwrap();
    let set_regex = Regex::new(r"SET (\w+) (\w+)").unwrap();
    let delete_regex = Regex::new(r"DEL (\w+)").unwrap();

    for stream in listener.incoming() {
        println!("KV server: Accepted connection");
        let stream = RefCell::new(stream.unwrap());
        let stream_ref = &stream;
        // TODO understand why "stream_ref.borrow_mut" fails but "stream_ref.borrow_mut.try_clone"
        // works
        let buf_reader = BufReader::new(stream_ref.borrow_mut().try_clone().unwrap());

        for line in buf_reader.lines() {
            let request_line_string = line.unwrap();

            if let Some((key, value)) = is_set_command(&request_line_string, &set_regex) {
                println!("KV server: SET {} = {}", key, value);
            } else if let Some(key) = is_get_command(&request_line_string, &get_regex) {
                println!("KV server: GET {}", key);
                stream_ref.borrow_mut().write_all(b"44\n").unwrap();
            } else if let Some(key) = is_del_command(&request_line_string, &delete_regex) {
                println!("KV server: DEL {}", key);
            } else {
                println!("KV server: uknown command {}", request_line_string)
            }
        }
    }
}
