# 28-08-2023

- Splitting the key namespace creates a problem. With the original design (primary/backups) the sequence number had the same semantics across all nodes. Now that each node is the owner of a portion of the namespace each one has it's own sequence.
- I can create one log file per node & namespace. So when a node replicates values from another node it will write those to a separate logfile, where a sequence is kept.

# 27-08-2023

- Thinking about splitting the key namespace across multiple nodes. I consider this options:

  - Make the client smart. The client (repl and webserver) have to know which KV nodes is responsible for the key they want to store.
  - Introduce a request router. The router sits in between the clients and the KV node and it's responsible for forwarding the request to the right KV node.

- Will go with smart clients as it seems easier to do. There's one less component to create, launch. There are no requests to be forwarded.
- For the first iteration I'm going to disable replication to limit the amount of moving pieces. For replication each node has to know which other nodes exist and to which one they can replicate.
- To find out which other nodes exist I could use gossip or a centralized metadata repository (e.g. zookeeper)
- The keys will be split based on the number of KV nodes (range / # KV nodes). The keys are assumed to only start with numbers or letters.
