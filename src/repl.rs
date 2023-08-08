use crate::{ChannelEnd, Command, Message};
use easy_repl::{command, CommandStatus, Repl};
use std::{
    cell::RefCell,
    io::{Write, BufReader, BufRead},
    net::TcpStream,
    sync::mpsc::{Receiver, Sender},
};

pub(crate) fn start_repl(tx: Sender<(Message, ChannelEnd)>, rx: Receiver<(Message, ChannelEnd)>) {
    let tx_cell = RefCell::new(tx);
    let tx_ref = &tx_cell;
    let rx_cell = RefCell::new(rx);
    let rx_ref = &rx_cell;

    // TODO: the connection might be refused if the server thread was scheduled later than the repl
    // thread
    let stream = RefCell::new(TcpStream::connect("localhost:1337").unwrap());
    let stream_ref = &stream;
    let mut reader = BufReader::new(stream_ref.borrow_mut().try_clone().unwrap());

    let mut repl = Repl::builder()
        .add(
            "SET",
            command! {
                "Set a value",
                (key: String, value: String) => |key: String, value: String| {
                    // TODO: can I pass a Message to write_all and have it serialized as a vector?
                    stream_ref.borrow_mut().write_all(format!("SET {key} {value}\n").as_bytes()).unwrap();
                    // tx_ref.borrow_mut().send((Message::Request(Command::Set { key, value }), ChannelEnd::Repl)).unwrap();
                    // rx_ref.borrow_mut().recv().unwrap();
                    Ok(CommandStatus::Done)
                }
            },
        )
        .add(
            "GET",
            command! {
                "Get a value",
                (key: String) => |key| {
                    stream_ref.borrow_mut().write_all(format!("GET {key}\n").as_bytes()).unwrap();
                    // tx_ref.borrow_mut().send((Message::Request(Command::Get { key }), ChannelEnd::Repl)).unwrap();
                    // let ( Message::Response(value), _ ) = rx_ref.borrow_mut().recv().unwrap() else { panic!() };

                    
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
                    tx_ref.borrow_mut().send((Message::Request(Command::Delete { key }), ChannelEnd::Repl)).unwrap();

                    Ok(CommandStatus::Done)
                }
            },
        )
        .build()
        .expect("Failed to create repl");

    repl.run().expect("Critical REPL error");
}
