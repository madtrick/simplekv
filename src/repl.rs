use crate::{ChannelEnd, Command, Message};
use easy_repl::{command, CommandStatus, Repl};
use std::{
    cell::RefCell,
    sync::mpsc::{Receiver, Sender},
};

pub(crate) fn start_repl(tx: Sender<(Message, ChannelEnd)>, rx: Receiver<(Message, ChannelEnd)>) {
    let tx_cell = RefCell::new(tx);
    let tx_ref = &tx_cell;

    let mut repl = Repl::builder()
        .add(
            "SET",
            command! {
                "Set a value",
                (key: String, value: String) => |key: String, value: String| {
                    tx_ref.borrow_mut().send((Message::Request(Some(Command::Set { key, value })), ChannelEnd::Repl)).unwrap();
                    Ok(CommandStatus::Done)
                }
            },
        )
        .add(
            "GET",
            command! {
                "Get a value",
                // TODO: replace &key with key
                (key: String) => |key| {
                    tx_ref.borrow_mut().send((Message::Request(Some(Command::Get { key })), ChannelEnd::Repl)).unwrap();
                    let ( Message::Response(Some(recv_message)), _ ) = rx.recv().unwrap() else { panic!() };
                    println!("Received in repl {:?}", recv_message);
                    // let ( Command::Get{key: key_value}, _ ) = recv_message else { panic!()};
                    // println!("{}", key_value);

                    Ok(CommandStatus::Done)
                }
            },
        )
        .add(
            "DEL",
            command! {
                "Delete a value",
                (key: String) => |key: String| {
                    tx_ref.borrow_mut().send((Message::Request(Some(Command::Delete { key })), ChannelEnd::Repl)).unwrap();

                    Ok(CommandStatus::Done)
                }
            },
        )
        .build()
        .expect("Failed to create repl");

    repl.run().expect("Critical REPL error");
}
