use common::*;

use color_eyre::eyre::Result;
use crossbeam::channel::{unbounded, Receiver, Sender, TryRecvError};
use serde::{Deserialize, Serialize};
use serde_json::Value;

struct RequestHandler {
    inner_node: Node,
    recieved_values: Vec<u64>,
    gossip_handler: Sender<GossipMsg>,
    stdout_sender: Sender<String>,
}

struct GossipManager {
    reciever: Receiver<GossipMsg>,
    gossip_queue: Vec<(Broadcast, String)>,
    stdout_sender: Sender<String>,
    node_id: String,
}

impl GossipManager {
    fn handle_gossip(mut self) -> Result<()> {
        loop {
            let msg = self.reciever.try_recv();

            match msg {
                Ok(GossipMsg::Gossip(b, dest)) => {
                    self.gossip_queue.push((b.clone(), dest.clone()));

                    self.send_gossip(b, dest);
                }
                Ok(GossipMsg::GotResponse(in_response_to)) => self
                    .gossip_queue
                    .retain(|(b, _)| b.msg_id != in_response_to),
                Err(TryRecvError::Disconnected) => break,
                Err(TryRecvError::Empty) => {
                    for (b, dest) in self.gossip_queue.iter() {
                        self.send_gossip(b.clone(), dest.clone());
                    }
                }
            };
        }

        Ok(())
    }

    fn send_gossip(&self, b: Broadcast, dest: String) {
        let m = Message {
            body: RequestBody::Broadcast(b),
            dest,
            src: self.node_id.to_owned(),
        };
        let output = serde_json::to_string(&m).unwrap();

        self.stdout_sender.send(output).unwrap();
    }
}

enum GossipMsg {
    Gossip(Broadcast, String),
    GotResponse(MsgId),
}

impl RequestHandler {
    fn gossip(&mut self, b: Broadcast, dest: &str) -> Result<()> {
        self.gossip_handler
            .send(GossipMsg::Gossip(b, dest.to_owned()))?;

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
    Topology { msg_id: MsgId, topology: Value },
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
                let peers = self.inner_node.peers.clone();
                let me_id = self.node_id().to_owned();

                for node in peers.iter().filter(|node| node != &&me_id) {
                    let msg_id = self.inner_node.generate_msg_id();
                    self.gossip(
                        Broadcast {
                            msg_id,
                            message: *message,
                        },
                        node,
                    )
                    .unwrap();
                }

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
            RequestBody::Topology { msg_id, .. } => Some(ResponseBody::Topology {
                msg_id: self.inner_node.generate_msg_id(),
                in_reply_to: *msg_id,
            }),
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
