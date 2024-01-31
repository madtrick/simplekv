use serde::{Deserialize, Serialize};
use std::{collections::HashMap, ops::RangeInclusive};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Command {
    Set { key: String, value: String },
    Delete { key: String },
    Get { key: String },
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Connect {
    pub from: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ConnectOk {
    pub map: HashMap<String, String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Message {
    Command(Command),
    ReplicationCommand(ReplicationCommand),
    Connect(Connect),
    ConnectOk(ConnectOk),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReplicationCommand {
    pub command: Command,
    pub sequence: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NamespaceAllocation {
    pub node: String,
    pub range: RangeInclusive<char>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Node {
    pub node_id: u8,
    pub address: String,
}
