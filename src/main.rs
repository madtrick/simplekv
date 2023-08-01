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
    let set_regex = Regex::new(r"(\w+)=(\w+)").unwrap();
    let del_regex = Regex::new(r"DEL (\w+)").unwrap();

    lines.for_each(|line| {
        if let Ok(line) = line {
            // This patter of `let x = foo else bar` is called let-else. https://rust-lang.github.io/rfcs/3137-let-else.html
            if let Some(capture) = set_regex.captures(line.as_str()) {
                println!("Key {}, Value {}", &capture[1], &capture[2]);
                map.insert(String::from(&capture[1]), String::from(&capture[2]));
            } else if let Some(capture) = del_regex.captures(line.as_str()) {
                println!("Del Key {}", &capture[1]);
                map.remove(&capture[1]);
            };

            // TODO: understand why I need to pass a reference to println
        }
    });

    map
}

fn main() {
    let map = init_from_logfile("log");
    let values = RefCell::new(map);
    let values_ref = &values;
    let file = RefCell::new(upsert_logfile("log"));
    let file_ref = &file;

    print_logfile("log");

    let mut repl = Repl::builder()
        .add(
            "SET",
            command! {
                "Set a value",
                (key: String, value: String) => |key: String, value: String| {
                    println!("Set {} = {}", key, value);
                    let mut map = values_ref.borrow_mut();
                    let mut file = file_ref.borrow_mut();

                    // TODO: how can I avoid cloning the key and value here?
                    map.insert(key.clone(), value.clone());

                    writeln!(file, "{}={}", key, value).unwrap();
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
        .add(
            "DEL",
            command! {
                "Delete a value",
                (key: String) => |key| {
                    // TODO: replace &key with key
                    let mut map = values_ref.borrow_mut();
                    let mut file = file_ref.borrow_mut();
                    map.remove(&key);

                    writeln!(&mut file, "DEL {}", key).unwrap();

                    Ok(CommandStatus::Done)
                }
            },
        )
        .build()
        .expect("Failed to create repl");

    repl.run().expect("Critical REPL error");
}
