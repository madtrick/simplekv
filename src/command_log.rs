use crate::Command;
use std::fs::File;
use std::io::Write;

pub(crate) struct CommandLog {
    file: File, // backing file to store the commands
}

fn __upsert_logfile(filename: &str) -> File {
    File::options()
        .append(true)
        .create(true)
        .open(filename)
        .unwrap()
}

impl CommandLog {
    pub fn new(filename: &str) -> CommandLog {
        CommandLog {
            file: __upsert_logfile(filename),
        }
    }

    pub fn append(&mut self, command: Command) {
        match command {
            Command::Set { key, value } => {
                println!("SET KEY {}, VALUE {}", key, value);
                writeln!(&mut self.file, "{}={}", key, value).unwrap();
            }
            Command::Delete { key } => {
                println!("DELETE KEY {}", key);
                writeln!(&mut self.file, "DEL {}", key).unwrap();
            }
            _ => panic!("Can't log this command"),
        }
    }
}
