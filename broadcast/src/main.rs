use common::*;

use color_eyre::eyre::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;

struct BroadcastNode {
    inner_node: Node,
    recieved_values: Vec<u64>,
    gossip_queue: Vec<Broadcast>,
}

impl BroadcastNode {
    fn gossip(&mut self, b: Broadcast, dest: &str) -> Result<()> {
        self.gossip_queue.push(b.clone());

        self.send_body(RequestBody::Broadcast(b), dest)
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

impl NodeIdable for BroadcastNode {
    fn node_id(&self) -> &str {
        self.inner_node.node_id()
    }
}

impl Handler for BroadcastNode {
    type RequestBody = RequestBody;
    type ResponseBody = ResponseBody;

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
                self.gossip_queue.retain(|m| m.msg_id != *in_reply_to);

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
    let node: BroadcastNode = BroadcastNode {
        inner_node: node,
        recieved_values: vec![],
        gossip_queue: vec![],
    };

    node.run()
}
