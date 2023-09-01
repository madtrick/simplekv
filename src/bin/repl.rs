use easy_repl::{command, CommandStatus, Repl};
use rustkv::{Command, Message, NamespaceAllocation};
use std::collections::HashMap;
use std::time::Duration;
use std::{
    cell::RefCell,
    io::{BufRead, BufReader, Write},
    net::TcpStream,
    ops::RangeInclusive,
};
use zookeeper::{Acl, CreateMode, WatchedEvent, Watcher, ZooKeeper};

fn select_key_owner(key: &str, owners: &HashMap<String, RangeInclusive<char>>) -> Option<String> {
    match key.chars().next() {
        None => None,
        Some(char) => {
            for (owner, range) in owners.iter() {
                if range.contains(&char) {
                    return Some(owner.clone());
                }
            }

            None
        }
    }
}

struct LoggingWatcher;
impl Watcher for LoggingWatcher {
    fn handle(&self, e: WatchedEvent) {
        // TODO: use the info! macro
        println!("{:?}", e)
    }
}

pub(crate) fn main() {
    let zk = ZooKeeper::connect("localhost:2181", Duration::from_secs(15), LoggingWatcher).unwrap();
    let (allocations_binary, _) = zk.get_data("/allocations", false).unwrap();
    let allocations =
        bincode::deserialize::<Vec<NamespaceAllocation>>(&allocations_binary).unwrap();

    // TODO: the connection might be refused if the server thread was scheduled later than the repl
    // thread
    let mut ownership = HashMap::new();
    let streams: &RefCell<HashMap<String, RefCell<TcpStream>>> = &RefCell::new(HashMap::new());

    for allocation in allocations {
        ownership.insert(allocation.node.clone(), allocation.range);
        streams.borrow_mut().insert(
            allocation.node.to_string(),
            RefCell::new(TcpStream::connect(allocation.node).unwrap()),
        );
    }

    let ownership = &RefCell::new(ownership);

    let mut repl = Repl::builder()
        .add(
            "SET",
            command! {
                "Set a value",
                (key: String, value: String) =>|key: String, value: String| {
                    match select_key_owner(&key, &ownership.borrow()) {
                        None => panic!("Unhandled key"),
                        Some(owner) => {
                            let result_stream = streams.borrow();
                            let stream = result_stream.get(&owner).unwrap();

                            // I have to finish the string with a \n because the receiving end is expecting
                            // a breakline terminated string
                            stream.borrow_mut().write_all(format!("{}\n", serde_json::to_string(&Message::Command(Command::Set { key, value })).unwrap()).as_bytes()).unwrap();

                            Ok(CommandStatus::Done)
                        }
                    }
                }
            },
        )
        .add(
            "GET",
            command! {
                "Get a value",
                (key: String) => |key: String| {
                    match select_key_owner(&key, &ownership.borrow()) {
                        None => panic!("Unhandled key"),
                        Some(owner) => {
                            // Have to split the borrow and getting the "owner" because
                            // of temporary values in Rust.
                            //
                            // This document gives a short explanation of when are temporaries
                            // introduced https://github.com/rust-lang/lang-team/blob/master/design-meeting-minutes/2023-03-15-temporary-lifetimes.md#when-are-temporaries-introduced
                            let streams_borrow = streams.borrow();
                            let stream = streams_borrow.get(&owner).unwrap();

                            stream.borrow_mut().write_all(format!("{}\n", serde_json::to_string(&Message::Command(Command::Get { key })).unwrap()).as_bytes()).unwrap();
                            let mut reader = BufReader::new(stream.borrow_mut().try_clone().unwrap());
                            let mut value = String::new();
                            reader.read_line(&mut value).unwrap();


                            println!("{}", value);


                            Ok(CommandStatus::Done)
                        }
                    }
                }
            },
        )
        .add(
            "DEL",
            command! {
                "Delete a value",
                (key: String) => |key: String| {
                    match select_key_owner(&key, &ownership.borrow()) {
                        None => panic!("Unhandled key"),
                        Some(owner) => {
                            let streams_borrow = streams.borrow();
                            let stream = streams_borrow.get(&owner).unwrap();

                            stream.borrow_mut().write_all(format!("{}\n", serde_json::to_string(&Message::Command(Command::Delete { key })).unwrap()).as_bytes()).unwrap();


                            Ok(CommandStatus::Done)
                        }
                    }
                }
            },
        )
        .build()
        .expect("Failed to create repl");

    repl.run().expect("Critical REPL error");
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    #[test]
    fn test_correct_routing() {
        let mut key_owners = HashMap::new();

        key_owners.insert("owner-1".to_string(), 'a'..='p');
        key_owners.insert("owner-2".to_string(), 'q'..='z');

        assert_eq!(
            super::select_key_owner("abc", &key_owners),
            Some("owner-1".to_string())
        );
        assert_eq!(
            super::select_key_owner("qr", &key_owners),
            Some("owner-2".to_string())
        );
        assert_eq!(
            super::select_key_owner("p", &key_owners),
            Some("owner-1".to_string())
        );
        assert_eq!(
            super::select_key_owner("z", &key_owners),
            Some("owner-2".to_string())
        );
    }
}
