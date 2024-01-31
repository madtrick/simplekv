# 03-09-2023

- Sometimes when I shutdown a node the coordinator doesn't get a watch notification. I create a persistent recursive watch and after adding a node I didn't get the notification (I checked with the ZK CLI and the node had been created)

# 02-09-2023

- Right now the allocation of namespace items to the nodes is harcoded in the coordinator. If a node were to crash, another one would have to be created on the same host and port to be able to receive traffic
- The connection to the new node from the REPL would have failed and only if the new node listens on the same host and port, would the repl be able to connect again.
- The new code could instead register in the coordinator, indicating which range wants to take over. The coordinator would indicate from which replicas to recover. The new node would register with the coordinator. The repl could be listening to changes in ZK and pick up the new node.
- If a node goes down, the repl could continue executing read-only queries against the replicas.
-

# 01-09-2023

- In the coordinator I'm checking if a zookeeper node exists or not before trying to create it. What kind of scenarios I'd have to consider if zookeeper was running across multiple nodes and there was the possibility of a split brain
- Though maybe the above point 👆 is not a problem because ZK is a CP system https://stackoverflow.com/questions/35387774/is-zookeeper-always-consistent-in-terms-of-cap-theorem. One side of the split can create the same node again.

# 29-08-2023

- Rust compile times on Docker can be too long. Use `cargo-chef` to speed them up https://www.lpalmieri.com/posts/fast-rust-docker-builds/

# 28-08-2023

- Splitting the key namespace creates a problem. With the original design (primary/backups) the sequence number had the same semantics across all nodes. Now that each node is the owner of a portion of the namespace each one has it's own sequence.
- I can create one log file per node & namespace. So when a node replicates values from another node it will write those to a separate logfile, where a sequence is kept.
- Each node will run multiple KV stores. One for the namespace it owns and one for each namespace it replicates.
- Having one sequence per namespace won't work if I move to consistent hashing. Or if I was to increase/decrease the size of the cluster. I would have to merge different namespaces where the same sequence number might have different meanings. This problem could be solved by having a coordination service that issues sequence numbers.

# 27-08-2023

- Thinking about splitting the key namespace across multiple nodes. I consider this options:

  - Make the client smart. The client (repl and webserver) have to know which KV nodes is responsible for the key they want to store.
  - Introduce a request router. The router sits in between the clients and the KV node and it's responsible for forwarding the request to the right KV node.

- Will go with smart clients as it seems easier to do. There's one less component to create, launch. There are no requests to be forwarded.
- For the first iteration I'm going to disable replication to limit the amount of moving pieces. For replication each node has to know which other nodes exist and to which one they can replicate.
- To find out which other nodes exist I could use gossip or a centralized metadata repository (e.g. zookeeper)
- The keys will be split based on the number of KV nodes (range / # KV nodes). The keys are assumed to only start with numbers or letters.
