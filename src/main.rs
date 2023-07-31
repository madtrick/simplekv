use easy_repl::{command, CommandStatus, Repl};
use std::{cell::RefCell, collections::HashMap};

fn main() {
    let values = RefCell::new(HashMap::new());
    let values_ref = &values;

    let mut repl = Repl::builder()
        .add(
            "SET",
            command! {
                "Set a value",
                (key: String, value: String) => |key, value| {
                    println!("Set {} = {}", key, value);
                    let mut map = values_ref.borrow_mut();

                    map.insert(key, value);
                    Ok(CommandStatus::Done)
                }
            },
        )
        .add(
            "GET",
            command! {
                "Get a value",
                (key: String) => |key| {
                    // TODO: replace &key with key
                    let map = values_ref.borrow();
                    let value = map.get(&key);

                    match value {
                        Some(value) => {
                            println!("Get {} = {}", key, value)
                        },
                        None => {
                            println!("Key not found")
                        }
                    }


                    Ok(CommandStatus::Done)
                }
            },
        )
        .build()
        .expect("Failed to create repl");

    repl.run().expect("Critical REPL error");
}
