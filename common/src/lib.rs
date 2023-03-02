use std::sync::{atomic::AtomicU64, Arc};

use color_eyre::eyre::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub struct Node {
    pub id: String,
    pub peers: Vec<String>,

    pub ids: Arc<IdGenerator>,
}

#[derive(Debug)]
pub struct IdGenerator {
    next_id: AtomicU64,
}

impl MsgIdAble for IdGenerator {
    fn generate_msg_id(&self) -> MsgId {
        self.next_id
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    }
}

impl Node {
    pub fn new(id: String, peers: Vec<String>, ids: Arc<IdGenerator>) -> Self {
        Self { id, peers, ids }
    }
}

pub type MsgId = u64;

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct Message<Body: Clone> {
    pub body: Body,
    pub dest: String,
    pub src: String,
}

pub trait NodeIdable {
    fn node_id(&self) -> &str;
}

impl NodeIdable for Node {
    fn node_id(&self) -> &str {
        &self.id
    }
}

pub trait MsgIdAble {
    fn generate_msg_id(&self) -> MsgId;
}

impl MsgIdAble for Node {
    fn generate_msg_id(&self) -> MsgId {
        self.ids.generate_msg_id()
    }
}

pub trait Handler: NodeIdable + Sized {
    type RequestBody: Clone + for<'a> Deserialize<'a>;
    type ResponseBody: Serialize + Clone;

    fn respond_to(&mut self, m: Message<Self::RequestBody>) -> Result<()> {
        let body = self.handle_request(&m.body);

        let Some(body) = body else {
            return Ok(());
        };

        self.send_body(body, &m.src)
    }

    fn send_message<Body: Serialize + Clone>(&mut self, m: Message<Body>) -> Result<()> {
        let output = serde_json::to_string(&m)?;

        eprintln!("Sending: {output}");
        println!("{output}");

        Ok(())
    }

    fn send_body<Body: Serialize + Clone>(&mut self, body: Body, dest: &str) -> Result<()> {
        let m = Message {
            body,
            dest: dest.to_owned(),
            src: self.node_id().to_owned(),
        };

        self.send_message(m)
    }

    fn handle_request(&mut self, m: &Self::RequestBody) -> Option<Self::ResponseBody>;

    fn handle_requests(mut self) -> Result<()> {
        let stdin = std::io::stdin();

        loop {
            let mut buffer = String::new();
            let bytes = stdin.read_line(&mut buffer)?;

            if bytes != 0 && !buffer.is_empty() {
                eprintln!("Received: {}", buffer);

                let m = serde_json::from_str::<Message<Self::RequestBody>>(&buffer)?;

                self.respond_to(m)?;
            }
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ErrorMsg {
    pub code: i64,
    pub in_reply_to: MsgId,
    pub text: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Init {
    pub msg_id: MsgId,
    pub node_id: String,
    pub node_ids: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct InitResponse {
    pub msg_id: MsgId,
    pub in_reply_to: MsgId,
}

impl Init {
    pub fn response(&self, new_msg_id: MsgId) -> InitResponse {
        InitResponse {
            msg_id: new_msg_id,
            in_reply_to: self.msg_id,
        }
    }
}

pub trait MakeNewNode: Sized {
    fn init(init_msg: String) -> Result<Self>;
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum InitBody {
    #[serde(rename = "init")]
    Init(Init),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum InitBodyResponse {
    #[serde(rename = "init_ok")]
    InitResp(InitResponse),
}

impl MakeNewNode for Node {
    fn init(init_msg: String) -> Result<Self> {
        let m = serde_json::from_str::<Message<InitBody>>(&init_msg)?;

        let Message {
            body: InitBody::Init(Init {
                node_id, node_ids, ..
            }),
            ..
        } = &m;

        let ids = IdGenerator {
            next_id: AtomicU64::new(0),
        };
        let ids = Arc::new(ids);

        let mut node = Node {
            id: node_id.clone(),
            peers: node_ids.clone(),
            ids,
        };

        node.respond_to(m)?;

        Ok(node)
    }
}

impl Handler for Node {
    type RequestBody = InitBody;

    type ResponseBody = InitBodyResponse;

    fn handle_request(&mut self, m: &Self::RequestBody) -> Option<Self::ResponseBody> {
        match m {
            InitBody::Init(init) => Some(InitBodyResponse::InitResp(
                init.response(self.generate_msg_id()),
            )),
        }
    }
}
