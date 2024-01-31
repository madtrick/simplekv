use std::sync::mpsc::channel;
use std::thread;
use std::time::Duration;
use zookeeper::AddWatchMode;
use zookeeper::{Acl, CreateMode, WatchedEvent, Watcher, ZooKeeper};

struct LoggingWatcher;
impl Watcher for LoggingWatcher {
    fn handle(&self, e: WatchedEvent) {
        // TODO: use the info! macro
        println!("{:?}", e)
    }
}

/*
* The coordinator coordinates the nodes that are available to accept
* client requests and also replication.
*
* ## Node limit
*
* **Assumption**. No more nodes than the expected by the coordinator will register.
*
* ## Node registration
*
* When a node starts it registers with the coordinator. As part of the registration it specifies
* which range it wishes to manage.
*
*
* **Assumptions**:
*   - no two nodes will try to manage the same range or overlapping ranges.
*   - the node has already created a node in ZK under `/nodes`. The full path will be
*   `/nodes/node-{node_id}`. The value of `node_id` is included in the registration request.
*
* ## Data replication
*
* When all the expected nodes have registered, the coordinator will send a message to each node
* telling which are the nodes it must use to replicate data. The coordinator will take into account
* if a node was already replicating data previously and keep the same assignment. When the node
* registers it indicates if it replicating a range previously and which one.
*
* ## Node disconnection
*
* The coordinator watches the ZK node `/nodes/node-{node_id}`. If the node is deleted the
* coordinator switches all allocations to read-only mode. Notice that there can be a
* concurrent-in-flight SET request to the node that disconnected/died or to any of the other nodes
* (which will reject the request if they can't replicate). The purpose of the read-only mode is to
* fail fast SET requests.
*
* When the system switches to read-only mode the coordinator also changes the allocations. It
* will update the allocations to assign ownership of the range belonging to the disconnected node
* to one of its replicas. Note that in a real life scenario this could lead to an increase in load
* in the replica that could trigger a cascading failure. A possible option would be to let clients
* round robing between the replicas.
* */

pub fn main() {
    let zk = ZooKeeper::connect("localhost:2181", Duration::from_secs(1), LoggingWatcher).unwrap();
    // TODO: handle coordinator restart when there are already nodes registered
    // NOTE: it seems possible that a node crashes and restarts quickly. On that case is also
    // possible that ZK will deliver first the creation of the ZK node before the deletion of the
    // previous one.

    env_logger::init();

    let (send, recv) = channel();
    let closure = move |event: WatchedEvent| send.send(()).unwrap();
    zk.add_watch("/nodes", AddWatchMode::PersistentRecursive, closure)
        .unwrap();

    let node_create = zk.create(
        "/nodes",
        vec![],
        Acl::open_unsafe().clone(),
        CreateMode::Persistent,
    );

    match node_create {
        Err(zookeeper::ZkError::NodeExists) => println!("Node exists"),
        Err(_) => panic!("Unexpected error"),
        Ok(_) => (),
    }

    for x in recv.iter() {
        println!("Event {:?}, path {:?}", x, x);
    }

    // zk.get_children("/nodes", true).unwrap();

    // let allocations_exist = zk.exists("/allocations", false);
    //
    // match allocations_exist {
    //     Err(_) => panic!("Unexpected error"),
    //     Ok(Some(_)) => (),
    //     Ok(None) => {
    //         println!("Create allocations");
    //         let allocations = vec![
    //             NamespaceAllocation {
    //                 node: String::from_str("localhost:1337").unwrap(),
    //                 range: 'a'..='h',
    //             },
    //             NamespaceAllocation {
    //                 node: String::from_str("localhost:1338").unwrap(),
    //                 range: 'i'..='q',
    //             },
    //             NamespaceAllocation {
    //                 node: String::from_str("localhost:1339").unwrap(),
    //                 range: 'r'..='z',
    //             },
    //         ];
    //
    //         println!("allocations raw {:?}", allocations);
    //
    //         println!(
    //             "allocations {:?}",
    //             bincode::deserialize::<Vec<NamespaceAllocation>>(
    //                 &bincode::serialize(&allocations).unwrap()
    //             )
    //         );
    //         zk.create(
    //             "/allocations",
    //             bincode::serialize(&allocations).unwrap(),
    //             Acl::open_unsafe().clone(),
    //             CreateMode::Persistent,
    //         )
    //         .unwrap();
    //     }
    // }

    loop {
        thread::sleep(Duration::from_secs(1800));
        println!("Awake");
    }
}
