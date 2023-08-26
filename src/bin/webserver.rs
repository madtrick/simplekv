use regex::Regex;
use rustkv::Command;
use std::net::TcpListener;
use std::{
    cell::RefCell,
    io::{BufRead, BufReader, Write},
    net::TcpStream,
};

pub(crate) fn main() {
    // TODO make the port a config option
    let listener = TcpListener::bind("localhost:3333").unwrap();
    let kv_port = 1338;
    let kv_stream = RefCell::new(TcpStream::connect(format!("localhost:{kv_port}")).unwrap());
    let kv_stream_ref = &kv_stream;
    let mut reader = BufReader::new(kv_stream_ref.borrow_mut().try_clone().unwrap());

    for stream in listener.incoming() {
        let mut stream = stream.unwrap();

        let buf_reader = BufReader::new(&mut stream);
        let request_line_string = buf_reader.lines().next().unwrap().unwrap();
        let request_line = request_line_string.as_str();
        let get_regex = Regex::new(r"GET /get/(\w+) .+").unwrap();
        let set_regex = Regex::new(r"GET /set/(\w+)/(\w+) .+").unwrap();
        let delete_regex = Regex::new(r"GET /del/(\w+) .+").unwrap();

        let contents: String = if let Some(capture) = set_regex.captures(request_line) {
            let key = capture[1].to_string();
            let value = capture[2].to_string();

            kv_stream_ref
                .borrow_mut()
                .write_all(
                    format!(
                        "{}\n",
                        serde_json::to_string(&Command::Set { key, value }).unwrap()
                    )
                    .as_bytes(),
                )
                .unwrap();
            "SET".to_string()
        } else if let Some(capture) = get_regex.captures(request_line) {
            let key = capture[1].to_string();
            kv_stream_ref
                .borrow_mut()
                .write_all(
                    format!(
                        "{}\n",
                        serde_json::to_string(&Command::Get { key }).unwrap()
                    )
                    .as_bytes(),
                )
                .unwrap();

            let mut value = String::new();
            reader.read_line(&mut value).unwrap();

            value
        } else if let Some(capture) = delete_regex.captures(request_line) {
            let key = capture[1].to_string();
            kv_stream_ref
                .borrow_mut()
                .write_all(
                    format!(
                        "{}\n",
                        serde_json::to_string(&Command::Delete { key }).unwrap()
                    )
                    .as_bytes(),
                )
                .unwrap();

            "DEL".to_string()
        } else {
            "UNKNOWN".to_string()
        };

        let contents_length = contents.len();
        let response =
            format!("HTTP/1.1 200 OK\r\nContent-Length: {contents_length}\r\n\r\n{contents}");

        stream.write_all(response.as_bytes()).unwrap();
    }
}
