use color_eyre::Result;
use rand::Rng;

use std::{collections::HashMap, sync::Arc, time::Instant};

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
    // gossip_queue: Vec<Job>,
    stdout_sender: Sender<String>,
    node_id: String,

    /// Nearest Peers
    topology: Vec<String>,

    ids: Arc<IdGenerator>,

    to_gossip: HashMap<String, Vec<Job>>,
}

impl GossipManager {
    pub fn new(reciever: Receiver<GossipMsg>, stdout_sender: Sender<String>, node: &Node) -> Self {
        Self {
            reciever,
            stdout_sender,
            node_id: node.node_id().to_owned(),
            topology: node.peers.clone(),
            ids: Arc::clone(&node.ids),
            to_gossip: HashMap::new(),
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
                    let jitter = rand::thread_rng().gen_range(0..1000);

                    let run_at = now + std::time::Duration::from_millis(700 + jitter);

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

                        self.to_gossip
                            .entry(dest.clone())
                            .or_insert_with(Vec::new)
                            .push(job);
                    }
                }
                Ok(GossipMsg::GotResponse(in_response_to)) => {
                    let keys: Vec<_> = self.to_gossip.keys().cloned().collect();

                    for k in keys {
                        let jobs = self.to_gossip.get_mut(&k).unwrap();
                        jobs.retain(|job| job.broadcast.msg_id != in_response_to);
                    }
                }
                Err(TryRecvError::Disconnected) => break,
                Err(TryRecvError::Empty) => {
                    let now = std::time::Instant::now();
                    let stdout_sender = &self.stdout_sender;

                    // for jobs in self.to_gossip.values() {
                    //     for job in iobs {
                    //         if job.run_at <= now {
                    //             job.send(stdout_sender, self.node_id.clone());
                    //         }
                    //     }
                    // }
                    let keys: Vec<_> = self.to_gossip.keys().cloned().collect();
                    for k in keys {
                        let jobs = self.to_gossip.get_mut(&k).unwrap();
                        if jobs.is_empty() {
                            continue;
                        }

                        let min_run_at = jobs.iter().min_by_key(|job| job.run_at).unwrap().run_at;
                        if min_run_at > now {
                            continue;
                        }

                        let m = Message {
                            body: RequestBody::BulkBroadcast {
                                broadcasts: jobs.iter().map(|job| job.broadcast.clone()).collect(),
                            },
                            dest: k.clone(),
                            src: self.node_id.clone(),
                        };
                        let output = serde_json::to_string(&m).unwrap();

                        stdout_sender.send(output).unwrap();

                        for j in jobs {
                            j.attempts += 1;

                            let delay = std::time::Duration::from_millis(j.attempts * 1000);
                            j.run_at = std::time::Instant::now() + delay;
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
