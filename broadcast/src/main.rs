use common::*;

use color_eyre::eyre::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;

struct BroadcastNode {
    inner_node: Node,
    recieved_values: Vec<u64>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
enum RequestBody {
    #[serde(rename = "broadcast")]
    Broadcast { msg_id: MsgId, message: u64 },
    #[serde(rename = "read")]
    Read { msg_id: MsgId },
    #[serde(rename = "topology")]
    Topology { msg_id: MsgId, topology: Value },
}

#[derive(Serialize, Deserialize, Debug)]
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

    fn create_response_body(&mut self, body: &RequestBody) -> ResponseBody {
        match body {
            RequestBody::Broadcast { msg_id, message } => {
                self.recieved_values.push(*message);

                ResponseBody::Broadcast {
                    msg_id: self.inner_node.generate_msg_id(),
                    in_reply_to: *msg_id,
                }
            }
            RequestBody::Read { msg_id } => ResponseBody::Read {
                msg_id: self.inner_node.generate_msg_id(),
                in_reply_to: *msg_id,
                messages: self.recieved_values.clone(),
            },
            RequestBody::Topology { msg_id, .. } => ResponseBody::Topology {
                msg_id: self.inner_node.generate_msg_id(),
                in_reply_to: *msg_id,
            },
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
    };

    node.run()
}
