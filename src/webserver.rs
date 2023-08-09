use regex::Regex;
use std::net::TcpListener;
use std::{
    cell::RefCell,
    io::{BufRead, BufReader, Write},
    net::TcpStream,
};

pub(crate) fn launch_webserver() {
    // TODO make the port a config option
    let listener = TcpListener::bind("localhost:3333").unwrap();
    let kv_stream = RefCell::new(TcpStream::connect("localhost:1337").unwrap());
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
            let key = &capture[1];
            let value = &capture[2];

            kv_stream_ref
                .borrow_mut()
                .write_all(format!("SET {key} {value}\n").as_bytes())
                .unwrap();
            // tx.send((
            //     Message::Request(Command::Set {
            //         key: capture[1].to_string(),
            //         value: capture[2].to_string(),
            //     }),
            //     ChannelEnd::WebServer,
            // ))
            // .unwrap();
            "SET".to_string()
        } else if let Some(capture) = get_regex.captures(request_line) {
            let key = &capture[1];
            kv_stream_ref
                .borrow_mut()
                .write_all(format!("GET {key}\n").as_bytes())
                .unwrap();
            // tx_ref.borrow_mut().send((Message::Request(Command::Get { key }), ChannelEnd::Repl)).unwrap();
            // let ( Message::Response(value), _ ) = rx_ref.borrow_mut().recv().unwrap() else { panic!() };

            let mut value = String::new();
            reader.read_line(&mut value).unwrap();
            // tx.send((
            //     Message::Request(Command::Get {
            //         key: capture[1].to_string(),
            //     }),
            //     ChannelEnd::WebServer,
            // ))
            // .unwrap();
            // let ( Message::Response(recv_message), _ ) = rx.recv().unwrap() else { panic!() };
            value
        } else if let Some(capture) = delete_regex.captures(request_line) {
            let key = &capture[1];
            kv_stream_ref
                .borrow_mut()
                .write_all(format!("DEL {key}\n").as_bytes())
                .unwrap();
            // tx.send((
            //     Message::Request(Command::Delete {
            //         key: capture[1].to_string(),
            //     }),
            //     ChannelEnd::WebServer,
            // ))
            // .unwrap();
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
