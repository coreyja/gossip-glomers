use std::collections::HashMap;

use common::*;

mod gossip;
pub use gossip::*;

use color_eyre::eyre::Result;
use crossbeam::channel::unbounded;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
enum RequestBody {
    #[serde(rename = "broadcast")]
    Broadcast(Broadcast),
    #[serde(rename = "bulk_broadcast")]
    BulkBroadcast { broadcasts: Vec<Broadcast> },
    #[serde(rename = "read")]
    Read { msg_id: MsgId },
    #[serde(rename = "topology")]
    Topology {
        msg_id: MsgId,
        topology: HashMap<String, Vec<String>>,
    },
    #[serde(rename = "broadcast_ok")]
    BroadcastOk { msg_id: MsgId, in_reply_to: MsgId },
    #[serde(rename = "bulk_broadcast_ok")]
    BulkBroadcastOk { msg_id: MsgId, in_reply_to: MsgId },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Broadcast {
    msg_id: MsgId,
    message: u64,
    #[serde(skip_serializing_if = "Vec::is_empty", default = "Default::default")]
    already_sent_to: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
enum ResponseBody {
    #[serde(rename = "broadcast_ok")]
    Broadcast { msg_id: MsgId, in_reply_to: MsgId },
    #[serde(rename = "bulk_broadcast_ok")]
    BulkBroadcast { msg_id: MsgId, in_reply_to: MsgId },
    #[serde(rename = "read_ok")]
    Read {
        msg_id: MsgId,
        in_reply_to: MsgId,
        messages: Vec<u64>,
    },
    #[serde(rename = "topology_ok")]
    Topology { msg_id: MsgId, in_reply_to: MsgId },
}

mod requests;
pub(crate) use requests::*;

fn main() -> Result<()> {
    let stdin = std::io::stdin();

    // Init the node BEFORE we start the loop. We know the first message MUST be an init message
    let mut buffer = String::new();
    stdin.read_line(&mut buffer)?;
    let node = Node::init(buffer)?;

    let (stdout_sender, stdout_receiver) = unbounded();

    let (gossip_sender, gossip_receiver) = unbounded();

    let gossip_manager = GossipManager::new(gossip_receiver, stdout_sender.clone(), &node);

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
