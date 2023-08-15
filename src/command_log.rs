use crate::kv::Command;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};

pub(crate) struct CommandLog {
    file: File,       // backing file to store the commands
    filename: String, // name of the file
    sequence: usize,
}

fn upsert_logfile(filename: &String) -> File {
    File::options()
        .append(true)
        .create(true)
        .open(filename)
        .unwrap()
}

fn count_lines(filename: &String) -> usize {
    // NOTE: I wanted to pass the `File` returned from `upsert_logfile` when I did so the
    // could would hang. The CommandLog code wouldn't execute pass the call to this
    // function.
    // TODO: read only the last line and get its sequence value
    BufReader::new(File::open(filename).unwrap())
        .lines()
        .count()
}

impl CommandLog {
    pub fn new(filename: String) -> CommandLog {
        let file = upsert_logfile(&filename);
        let sequence = count_lines(&filename);

        println!("Sequence {}", sequence);

        CommandLog {
            file,
            filename,
            sequence,
        }
    }

    pub fn append(&mut self, command: &Command) -> usize {
        match command {
            Command::Set { ref key, ref value } => {
                writeln!(&mut self.file, "{}#{}={}", self.sequence, key, value).unwrap();
            }
            Command::Delete { ref key } => {
                writeln!(&mut self.file, "{}#DEL {}", self.sequence, key).unwrap();
            }
            _ => panic!("Can't log this command"),
        }

        self.sequence += 1;
        self.sequence
    }

    pub fn filename(&self) -> &String {
        &self.filename
    }
}
