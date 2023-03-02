use color_eyre::Result;

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

pub enum GossipMsg {
    Gossip(u64),
    Topology(Vec<String>),
    GotResponse(MsgId),
}
