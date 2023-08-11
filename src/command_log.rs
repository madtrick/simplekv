use crate::Command;
use std::fs::File;
use std::io::Write;

pub(crate) struct CommandLog {
    file: File,       // backing file to store the commands
    filename: String, // name of the file
}

fn upsert_logfile(filename: &String) -> File {
    File::options()
        .append(true)
        .create(true)
        .open(filename)
        .unwrap()
}

impl CommandLog {
    pub fn new(filename: String) -> CommandLog {
        CommandLog {
            file: upsert_logfile(&filename),
            filename,
        }
    }

    pub fn append(&mut self, command: Command) {
        match command {
            Command::Set { key, value } => {
                writeln!(&mut self.file, "{}={}", key, value).unwrap();
            }
            Command::Delete { key } => {
                writeln!(&mut self.file, "DEL {}", key).unwrap();
            }
            _ => panic!("Can't log this command"),
        }
    }

    pub fn filename(&self) -> &String {
        &self.filename
    }
}
