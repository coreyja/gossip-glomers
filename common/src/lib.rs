use color_eyre::eyre::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub struct Node {
    pub id: String,
    pub peers: Vec<String>,

    next_msg_id: MsgId,
}

impl Node {
    pub fn new(id: String, peers: Vec<String>) -> Self {
        Self {
            id,
            peers,
            next_msg_id: 0,
        }
    }
}

pub type MsgId = u64;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Message<Body> {
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
    fn generate_msg_id(&mut self) -> MsgId;
}

impl MsgIdAble for Node {
    fn generate_msg_id(&mut self) -> MsgId {
        let next = self.next_msg_id;
        self.next_msg_id += 1;

        next
    }
}

pub trait Handler: NodeIdable + Sized {
    type RequestBody: for<'a> Deserialize<'a>;
    type ResponseBody: Serialize;

    fn handle_message(&mut self, m: Message<Self::RequestBody>) -> Result<()> {
        let resp = self.create_response(m);

        let output = serde_json::to_string(&resp)?;

        println!("{output}");

        Ok(())
    }

    fn create_response(&mut self, m: Message<Self::RequestBody>) -> Message<Self::ResponseBody> {
        let body = self.create_response_body(&m.body);

        Message {
            body,
            dest: m.src.clone(),
            src: self.node_id().to_owned(),
        }
    }

    fn create_response_body(&mut self, m: &Self::RequestBody) -> Self::ResponseBody;

    fn run(mut self) -> Result<()> {
        let stdin = std::io::stdin();

        loop {
            let mut buffer = String::new();
            let bytes = stdin.read_line(&mut buffer)?;

            if bytes != 0 && !buffer.is_empty() {
                eprintln!("Received: {}", buffer);

                let m = serde_json::from_str::<Message<Self::RequestBody>>(&buffer)?;

                self.handle_message(m)?;
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

#[derive(Serialize, Deserialize, Debug)]
pub struct Init {
    pub msg_id: MsgId,
    pub node_id: String,
    pub node_ids: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
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

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum InitBody {
    #[serde(rename = "init")]
    Init(Init),
}

#[derive(Serialize, Deserialize, Debug)]
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

        let mut node = Node {
            id: node_id.clone(),
            peers: node_ids.clone(),

            next_msg_id: 0,
        };

        node.handle_message(m)?;

        Ok(node)
    }
}

impl Handler for Node {
    type RequestBody = InitBody;

    type ResponseBody = InitBodyResponse;

    fn create_response_body(&mut self, m: &Self::RequestBody) -> Self::ResponseBody {
        match m {
            InitBody::Init(init) => {
                InitBodyResponse::InitResp(init.response(self.generate_msg_id()))
            }
        }
    }
}
