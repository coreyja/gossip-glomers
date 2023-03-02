use common::*;

use color_eyre::eyre::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;

struct UniqueIdNode {
    inner_node: Node,
    next_id: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
enum RequestBody {
    #[serde(rename = "generate")]
    Generate { msg_id: MsgId },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
enum ResponseBody {
    #[serde(rename = "generate_ok")]
    Generate {
        id: Value,
        msg_id: MsgId,
        in_reply_to: MsgId,
    },
}

impl NodeIdable for UniqueIdNode {
    fn node_id(&self) -> &str {
        self.inner_node.node_id()
    }
}

impl Handler for UniqueIdNode {
    type RequestBody = RequestBody;
    type ResponseBody = ResponseBody;

    fn handle_request(&mut self, body: &RequestBody) -> Option<ResponseBody> {
        let next = self.next_id;
        self.next_id += 1;

        let id_vec: Vec<Value> = vec![
            Value::String(self.inner_node.id.clone()),
            Value::Number(next.into()),
        ];

        eprintln!("Node {} generated id {:?}", self.inner_node.id, &id_vec);

        Some(match body {
            RequestBody::Generate { msg_id } => ResponseBody::Generate {
                id: id_vec.into(),
                msg_id: self.inner_node.generate_msg_id(),
                in_reply_to: *msg_id,
            },
        })
    }
}

fn main() -> Result<()> {
    let stdin = std::io::stdin();

    // Init the node BEFORE we start the loop. We know the first message MUST be an init message
    let mut buffer = String::new();
    stdin.read_line(&mut buffer)?;

    let node = Node::init(buffer)?;
    let node: UniqueIdNode = UniqueIdNode {
        inner_node: node,
        next_id: 0,
    };

    node.handle_requests()
}
