use crate::{ChannelEnd, Command, Message};
use easy_repl::{command, CommandStatus, Repl};
use std::{
    cell::RefCell,
    sync::mpsc::{Receiver, Sender},
};

pub(crate) fn start_repl(tx: Sender<(Message, ChannelEnd)>, rx: Receiver<(Message, ChannelEnd)>) {
    let tx_cell = RefCell::new(tx);
    let tx_ref = &tx_cell;
    let rx_cell = RefCell::new(rx);
    let rx_ref = &rx_cell;

    let mut repl = Repl::builder()
        .add(
            "SET",
            command! {
                "Set a value",
                (key: String, value: String) => |key: String, value: String| {
                    tx_ref.borrow_mut().send((Message::Request(Command::Set { key, value }), ChannelEnd::Repl)).unwrap();
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
                    tx_ref.borrow_mut().send((Message::Request(Command::Get { key }), ChannelEnd::Repl)).unwrap();
                    let ( Message::Response(value), _ ) = rx_ref.borrow_mut().recv().unwrap() else { panic!() };

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
