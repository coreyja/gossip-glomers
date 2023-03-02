use common::*;

use color_eyre::eyre::Result;
use serde::{Deserialize, Serialize};

struct EchoNode(Node);

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
enum RequestBody {
    #[serde(rename = "echo")]
    Echo(Echo),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
enum ResponseBody {
    #[serde(rename = "echo_ok")]
    EchoResponse(EchoResponse),
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
struct Echo {
    msg_id: MsgId,
    echo: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
struct EchoResponse {
    msg_id: MsgId,
    echo: String,
    in_reply_to: MsgId,
}

impl NodeIdable for EchoNode {
    fn node_id(&self) -> &str {
        self.0.node_id()
    }
}

impl Handler for EchoNode {
    type RequestBody = RequestBody;
    type ResponseBody = ResponseBody;

    fn handle_request(&mut self, body: &RequestBody) -> Option<ResponseBody> {
        let new_msg_id = self.0.generate_msg_id();

        match body {
            RequestBody::Echo(e) => Some(ResponseBody::EchoResponse(EchoResponse {
                msg_id: new_msg_id,
                echo: e.echo.clone(),
                in_reply_to: e.msg_id,
            })),
        }
    }
}

fn main() -> Result<()> {
    let stdin = std::io::stdin();

    // Init the node BEFORE we start the loop. We know the first message MUST be an init message
    let mut buffer = String::new();
    stdin.read_line(&mut buffer)?;

    let node = Node::init(buffer)?;
    let node: EchoNode = EchoNode(node);

    node.handle_requests()
}
