use color_eyre::Result;
use common::{Handler, Message, MsgIdAble, Node, NodeIdable};
use crossbeam::channel::Sender;
use serde::Serialize;

use crate::{Broadcast, GossipMsg, RequestBody, ResponseBody};

pub(crate) struct RequestHandler {
    pub inner_node: Node,
    pub recieved_values: Vec<u64>,
    pub gossip_handler: Sender<GossipMsg>,
    pub stdout_sender: Sender<String>,
}

impl RequestHandler {
    fn gossip(&mut self, b: u64) -> Result<()> {
        self.gossip_handler.send(GossipMsg::Gossip(b))?;

        Ok(())
    }
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
