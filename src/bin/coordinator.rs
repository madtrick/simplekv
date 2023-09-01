use rustkv::NamespaceAllocation;
use std::str::FromStr;
use std::time::Duration;
use zookeeper::ZkError;
use zookeeper::{Acl, CreateMode, WatchedEvent, Watcher, ZooKeeper};

struct LoggingWatcher;
impl Watcher for LoggingWatcher {
    fn handle(&self, e: WatchedEvent) {
        // TODO: use the info! macro
        println!("{:?}", e)
    }
}

pub fn main() {
    let zk = ZooKeeper::connect("localhost:2181", Duration::from_secs(15), LoggingWatcher).unwrap();
    let allocations_exist = zk.exists("/allocations", false);

    match allocations_exist {
        Err(_) => panic!("Unexpected error"),
        Ok(Some(_)) => (),
        Ok(None) => {
            println!("Create allocations");
            let allocations = vec![
                NamespaceAllocation {
                    node: String::from_str("localhost:1337").unwrap(),
                    range: 'a'..='h',
                },
                NamespaceAllocation {
                    node: String::from_str("localhost:1338").unwrap(),
                    range: 'i'..='q',
                },
                NamespaceAllocation {
                    node: String::from_str("localhost:1339").unwrap(),
                    range: 'r'..='z',
                },
            ];

            println!("allocations raw {:?}", allocations);

            println!(
                "allocations {:?}",
                bincode::deserialize::<Vec<NamespaceAllocation>>(
                    &bincode::serialize(&allocations).unwrap()
                )
            );
            zk.create(
                "/allocations",
                bincode::serialize(&allocations).unwrap(),
                Acl::open_unsafe().clone(),
                CreateMode::Persistent,
            )
            .unwrap();
        }
    }
}
