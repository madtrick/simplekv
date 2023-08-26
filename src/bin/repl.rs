use easy_repl::{command, CommandStatus, Repl};
use rustkv::{Command, Message};
use std::{
    cell::RefCell,
    io::{BufRead, BufReader, Write},
    net::TcpStream,
};

pub(crate) fn main() {
    // TODO: the connection might be refused if the server thread was scheduled later than the repl
    // thread
    let kv_port = 1338;
    let stream = RefCell::new(TcpStream::connect(format!("localhost:{kv_port}")).unwrap());
    let stream_ref = &stream;
    let mut reader = BufReader::new(stream_ref.borrow_mut().try_clone().unwrap());

    let mut repl = Repl::builder()
        .add(
            "SET",
            command! {
                "Set a value",
                (key: String, value: String) =>|key: String, value: String| {
                    // I have to finish the string with a \n because the receiving end is expecting
                    // a breakline terminated string
                    stream_ref.borrow_mut().write_all(format!("{}\n", serde_json::to_string(&Message::Command(Command::Set { key, value })).unwrap()).as_bytes()).unwrap();

                    Ok(CommandStatus::Done)
                }
            },
        )
        .add(
            "GET",
            command! {
                "Get a value",
                (key: String) => |key| {
                    stream_ref.borrow_mut().write_all(format!("{}\n", serde_json::to_string(&Message::Command(Command::Get { key })).unwrap()).as_bytes()).unwrap();

                    let mut value = String::new();
                    reader.read_line(&mut value).unwrap();


                    println!("{}", value);

                    Ok(CommandStatus::Done)
                }
            },
        )
        .add(
            "DEL",
            command! {
                "Delete a value",
                (key: String) => |key: String| {
                    stream_ref.borrow_mut().write_all(format!("{}\n", serde_json::to_string(&Message::Command(Command::Delete { key })).unwrap()).as_bytes()).unwrap();

                    Ok(CommandStatus::Done)
                }
            },
        )
        .build()
        .expect("Failed to create repl");

    repl.run().expect("Critical REPL error");
}