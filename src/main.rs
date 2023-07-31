use easy_repl::{command, CommandStatus, Repl};
use regex::Regex;
use std::io::{BufRead, BufReader, Write};
use std::{cell::RefCell, collections::HashMap, fs::File};

fn upsert_logfile(filename: &str) -> File {
    File::options()
        .append(true)
        .create(true)
        .open(filename)
        .unwrap()
}

fn print_logfile(filename: &str) {
    // TODO: I wanted to do `File::open(filename)?` but it won't compile. Why?
    let file = File::open(filename).unwrap();
    let lines = BufReader::new(file).lines();
    let regex = Regex::new(r"(\w+)=(\w+)").unwrap();

    lines.for_each(|line| {
        if let Ok(line) = line {
            let Some(capture) = regex.captures(line.as_str()) else { return };

            println!("Key {}, Value {}", &capture[1], &capture[2])
        }
    })
}

fn init_from_logfile(filename: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let file = File::open(filename).unwrap();
    let lines = BufReader::new(file).lines();
    let regex = Regex::new(r"(\w+)=(\w+)").unwrap();

    lines.for_each(|line| {
        if let Ok(line) = line {
            // This patter of `let x = foo else bar` is called let-else. https://rust-lang.github.io/rfcs/3137-let-else.html
            let Some(capture) = regex.captures(line.as_str()) else { return };

            // TODO: understand why I need to pass a reference to println
            println!("Key {}, Value {}", &capture[1], &capture[2]);
            map.insert(String::from(&capture[1]), String::from(&capture[2]));
        }
    });

    map
}

fn main() {
    let map = init_from_logfile("log");
    let values = RefCell::new(map);
    let values_ref = &values;
    let mut file = upsert_logfile("log");

    print_logfile("log");

    let mut repl = Repl::builder()
        .add(
            "SET",
            command! {
                "Set a value",
                (key: String, value: String) => |key: String, value: String| {
                    println!("Set {} = {}", key, value);
                    let mut map = values_ref.borrow_mut();

                    // TODO: how can I avoid cloning the key and value here?
                    map.insert(key.clone(), value.clone());

                    writeln!(&mut file, "{}={}", key, value).unwrap();
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
