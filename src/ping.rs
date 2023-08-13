use std::io::{BufRead, BufReader, Write};
use std::time::Duration;
use std::{net::TcpStream, thread};

pub struct StartPingOptions {
    pub peers: Vec<String>,
}

struct Ping {
    stream: TcpStream,
}

impl Ping {
    fn new(peer: String) -> Self {
        println!("PEER {}", &peer);
        let stream = TcpStream::connect(&peer).unwrap();

        Ping { stream }
    }

    fn start(mut self) {
        // TODO: why can't I create a reader instance at the same time as the stream
        // and have it read the incoming data. If I do it, the data doesn't seem to be consumed
        // (PONG messages are not processed).
        let mut reader = BufReader::new(self.stream.try_clone().unwrap());
        self.stream.write_all(b"PING\n").unwrap();
        let mut value = String::new();
        // TODO: timeout when waiting for a response
        reader.read_line(&mut value).unwrap();

        if value != "PONG\n" {
            panic!("Unexpected reply {:? } to 'PING' request", value);
        }

        thread::sleep(Duration::new(5, 0));

        self.start();
    }
}

pub fn start_ping(options: StartPingOptions) {
    let mut handles = Vec::new();

    for peer in options.peers {
        println!("peer {}", &peer);
        handles.push(thread::spawn(|| {
            let ping = Ping::new(peer);
            ping.start()
        }));
    }
}
