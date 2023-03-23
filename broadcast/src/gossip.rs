use color_eyre::Result;
use rand::Rng;

use std::{sync::Arc, time::Instant};

use common::{IdGenerator, Message, MsgId, MsgIdAble, Node, NodeIdable};
use crossbeam::channel::{Receiver, Sender, TryRecvError};

use crate::{Broadcast, RequestBody, RequestHandler};

#[derive(Debug, Clone)]
pub struct Job {
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

        let delay = std::time::Duration::from_millis(self.attempts * 1000);
        self.run_at = std::time::Instant::now() + delay;
    }
}

pub struct GossipManager {
    reciever: Receiver<GossipMsg>,
    gossip_queue: Vec<Job>,
    stdout_sender: Sender<String>,
    node_id: String,

    /// Nearest Peers
    topology: Vec<String>,

    ids: Arc<IdGenerator>,
}

impl GossipManager {
    pub fn new(reciever: Receiver<GossipMsg>, stdout_sender: Sender<String>, node: &Node) -> Self {
        Self {
            reciever,
            gossip_queue: Vec::new(),
            stdout_sender,
            node_id: node.node_id().to_owned(),
            topology: node.peers.clone(),
            ids: Arc::clone(&node.ids),
        }
    }

    pub fn handle_gossip(mut self) -> Result<()> {
        loop {
            let msg = self.reciever.try_recv();

            match msg {
                Ok(GossipMsg::Topology(topology)) => {
                    // self.topology = topology;
                }
                Ok(GossipMsg::Gossip {
                    msg,
                    already_sent_to,
                }) => {
                    // Lets see if our gossip queue can be trimmed based on this info
                    // self.gossip_queue.retain(|in_queue| {
                    //     in_queue.broadcast.message != msg
                    //         && !already_sent_to.contains(&in_queue.dest)
                    // });

                    let now = std::time::Instant::now();
                    let jitter = rand::thread_rng().gen_range(0..100);

                    let run_at = now + std::time::Duration::from_millis(100 + jitter);

                    let to_send_to = self
                        .topology
                        .iter()
                        .filter(|d| !already_sent_to.contains(d));

                    let mut already_sent_to = already_sent_to.clone();
                    already_sent_to.append(&mut to_send_to.clone().cloned().collect());

                    for dest in to_send_to {
                        let broadcast = Broadcast {
                            msg_id: self.ids.generate_msg_id(),
                            message: msg,
                            already_sent_to: already_sent_to.clone(),
                        };
                        let job = Job {
                            broadcast: broadcast.clone(),
                            dest: dest.clone(),
                            run_at,
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

pub enum GossipMsg {
    Gossip {
        msg: u64,
        already_sent_to: Vec<String>,
    },
    Topology(Vec<String>),
    GotResponse(MsgId),
}
