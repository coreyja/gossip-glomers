use std::{collections::HashMap, sync::Arc, time::Instant};

use common::*;

use color_eyre::eyre::Result;
use crossbeam::channel::{unbounded, Receiver, Sender, TryRecvError};
use serde::{Deserialize, Serialize};

struct RequestHandler {
    inner_node: Node,
    recieved_values: Vec<u64>,
    gossip_handler: Sender<GossipMsg>,
    stdout_sender: Sender<String>,
}

#[derive(Debug, Clone)]
struct Job {
    broadcast: Broadcast,
    dest: String,
    run_at: Instant,
    attempts: u64,
}

impl Job {
    fn send(&mut self, stdout_sender: &Sender<String>, node_id: String) {
        let m = Message {
            body: RequestBody::Broadcast(self.broadcast.clone()),
            dest: self.dest.clone(),
            src: node_id,
        };
        let output = serde_json::to_string(&m).unwrap();

        stdout_sender.send(output).unwrap();

        self.attempts += 1;

        // // attempts=1 -> 10ms
        // // attempts=2 -> 160ms
        // // attempts=3 -> 810ms
        // let backoff_delay = self.attempts.pow(4) * 10;

        // const CONSTANT_DELAY_MS: u64 = 300;
        // eprintln!("(attempt {}", self.attempts);

        // let delay = std::time::Duration::from_millis(CONSTANT_DELAY_MS + backoff_delay);

        let delay = std::time::Duration::from_millis(self.attempts * 200);
        self.run_at = std::time::Instant::now() + delay;
    }
}

struct GossipManager {
    reciever: Receiver<GossipMsg>,
    gossip_queue: Vec<Job>,
    stdout_sender: Sender<String>,
    node_id: String,

    /// Nearest Peers
    topology: Vec<String>,

    ids: Arc<IdGenerator>,
}

impl GossipManager {
    fn handle_gossip(mut self) -> Result<()> {
        loop {
            let msg = self.reciever.try_recv();

            match msg {
                Ok(GossipMsg::Topology(topology)) => {
                    self.topology = topology;
                }
                Ok(GossipMsg::Gossip(msg)) => {
                    let now = std::time::Instant::now();

                    for dest in self.topology.iter().filter(|d| d != &&self.node_id) {
                        let broadcast = Broadcast {
                            msg_id: self.ids.generate_msg_id(),
                            message: msg,
                        };
                        let job = Job {
                            broadcast: broadcast.clone(),
                            dest: dest.clone(),
                            run_at: now,
                            attempts: 0,
                        };
                        self.gossip_queue.push(job);
                    }
                }
                Ok(GossipMsg::GotResponse(in_response_to)) => self
                    .gossip_queue
                    .retain(|job| job.broadcast.msg_id != in_response_to),
                Err(TryRecvError::Disconnected) => break,
                Err(TryRecvError::Empty) => {
                    let now = std::time::Instant::now();
                    let stdout_sender = &self.stdout_sender;

                    for job in self.gossip_queue.iter_mut() {
                        if job.run_at <= now {
                            job.send(stdout_sender, self.node_id.clone());
                        }
                    }
                }
            };
        }

        Ok(())
    }
}

enum GossipMsg {
    Gossip(u64),
    Topology(Vec<String>),
    GotResponse(MsgId),
}

impl RequestHandler {
    fn gossip(&mut self, b: u64) -> Result<()> {
        self.gossip_handler.send(GossipMsg::Gossip(b))?;

        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
enum RequestBody {
    #[serde(rename = "broadcast")]
    Broadcast(Broadcast),
    #[serde(rename = "read")]
    Read { msg_id: MsgId },
    #[serde(rename = "topology")]
    Topology {
        msg_id: MsgId,
        topology: HashMap<String, Vec<String>>,
    },
    #[serde(rename = "broadcast_ok")]
    BroadcastOk { msg_id: MsgId, in_reply_to: MsgId },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Broadcast {
    msg_id: MsgId,
    message: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
enum ResponseBody {
    #[serde(rename = "broadcast_ok")]
    Broadcast { msg_id: MsgId, in_reply_to: MsgId },
    #[serde(rename = "read_ok")]
    Read {
        msg_id: MsgId,
        in_reply_to: MsgId,
        messages: Vec<u64>,
    },
    #[serde(rename = "topology_ok")]
    Topology { msg_id: MsgId, in_reply_to: MsgId },
}

impl NodeIdable for RequestHandler {
    fn node_id(&self) -> &str {
        self.inner_node.node_id()
    }
}

impl Handler for RequestHandler {
    type RequestBody = RequestBody;
    type ResponseBody = ResponseBody;

    fn send_message<Body: Serialize + Clone>(&mut self, m: Message<Body>) -> Result<()> {
        let output = serde_json::to_string(&m)?;

        self.stdout_sender.send(output)?;

        Ok(())
    }

    fn handle_request(&mut self, body: &RequestBody) -> Option<ResponseBody> {
        match body {
            RequestBody::Broadcast(Broadcast { msg_id, message }) => {
                if self.recieved_values.contains(message) {
                    return Some(ResponseBody::Broadcast {
                        msg_id: self.inner_node.generate_msg_id(),
                        in_reply_to: *msg_id,
                    });
                }

                self.recieved_values.push(*message);

                self.gossip(*message).unwrap();

                Some(ResponseBody::Broadcast {
                    msg_id: self.inner_node.generate_msg_id(),
                    in_reply_to: *msg_id,
                })
            }
            RequestBody::Read { msg_id } => Some(ResponseBody::Read {
                msg_id: self.inner_node.generate_msg_id(),
                in_reply_to: *msg_id,
                messages: self.recieved_values.clone(),
            }),
            RequestBody::Topology { msg_id, topology } => {
                let this_node_topology = topology.get(self.node_id()).unwrap().clone();
                self.gossip_handler
                    .send(GossipMsg::Topology(this_node_topology))
                    .unwrap();

                Some(ResponseBody::Topology {
                    msg_id: self.inner_node.generate_msg_id(),
                    in_reply_to: *msg_id,
                })
            }
            // We will get BroadcastOK message from the peers we gossip to
            RequestBody::BroadcastOk { in_reply_to, .. } => {
                self.gossip_handler
                    .send(GossipMsg::GotResponse(*in_reply_to))
                    .unwrap();

                None
            }
        }
    }
}

fn main() -> Result<()> {
    let stdin = std::io::stdin();

    // Init the node BEFORE we start the loop. We know the first message MUST be an init message
    let mut buffer = String::new();
    stdin.read_line(&mut buffer)?;
    let node = Node::init(buffer)?;

    let (stdout_sender, stdout_receiver) = unbounded();

    let (gossip_sender, gossip_receiver) = unbounded();

    let gossip_manager = GossipManager {
        reciever: gossip_receiver,
        gossip_queue: vec![],
        stdout_sender: stdout_sender.clone(),
        node_id: node.node_id().to_owned(),
        topology: node.peers.clone(),
        ids: Arc::clone(&node.ids),
    };

    let request_handler = RequestHandler {
        inner_node: node,
        recieved_values: vec![],
        gossip_handler: gossip_sender,
        stdout_sender,
    };

    let request_thread_handle = std::thread::spawn(|| request_handler.handle_requests());
    let gossip_join_handle = std::thread::spawn(move || gossip_manager.handle_gossip());
    let stdout_join_handle = std::thread::spawn(move || {
        stdout_receiver.iter().for_each(|output| {
            eprintln!("Sending: {output}");
            println!("{output}");
        });
    });

    request_thread_handle.join().unwrap()?;
    gossip_join_handle.join().unwrap()?;
    stdout_join_handle.join().unwrap();

    Ok(())
}
