use color_eyre::eyre::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug)]
struct Node {
    id: String,
    peers: Vec<String>,

    next_msg_id: MsgId,
}

fn main() -> Result<()> {
    let stdin = std::io::stdin();

    // Init the node BEFORE we start the loop. We know the first message MUST be an init message
    let mut buffer = String::new();
    stdin.read_line(&mut buffer)?;
    let m = serde_json::from_str::<Message<RequestBody>>(&buffer)?;

    let Message { body: RequestBody::Init(Init{node_id, node_ids, ..}), .. } = &m else {
        panic!("Expected init message, got {:?}", m);
    };

    let mut node = Node {
        id: node_id.clone(),
        peers: node_ids.clone(),

        next_msg_id: 0,
    };

    node.handle_message(m)?;

    loop {
        let mut buffer = String::new();
        let bytes = stdin.read_line(&mut buffer)?;

        if bytes != 0 && !buffer.is_empty() {
            eprintln!("Received: {}", buffer);

            let m = serde_json::from_str::<Message<RequestBody>>(&buffer)?;

            node.handle_message(m)?;
        }
    }
}

impl Node {
    fn create_response(&mut self, m: Message<RequestBody>) -> Message<ResponseBody> {
        let body = self.create_response_body(&m);

        Message {
            body,
            dest: m.src.clone(),
            src: self.id.clone(),
        }
    }

    fn create_response_body(&mut self, m: &Message<RequestBody>) -> ResponseBody {
        let new_msg_id = self.generate_msg_id();

        match &m.body {
            RequestBody::Echo(e) => e.response(new_msg_id),
            RequestBody::Init(e) => e.response(new_msg_id),
        }
    }

    fn handle_message(&mut self, m: Message<RequestBody>) -> Result<()> {
        let resp = self.create_response(m);

        let output = serde_json::to_string(&resp)?;

        println!("{output}");

        Ok(())
    }

    fn generate_msg_id(&mut self) -> MsgId {
        let next = self.next_msg_id;
        self.next_msg_id += 1;

        next
    }
}

type MsgId = u64;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct Message<Body> {
    body: Body,
    dest: String,
    src: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
enum RequestBody {
    #[serde(rename = "echo")]
    Echo(Echo),
    #[serde(rename = "init")]
    Init(Init),
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
enum ResponseBody {
    #[serde(rename = "echo_ok")]
    EchoResponse(EchoResponse),
    #[serde(rename = "error")]
    ErrorMsg(ErrorMsg),
    #[serde(rename = "init_ok")]
    InitResponse(InitResponse),
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct Echo {
    msg_id: MsgId,
    echo: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct EchoResponse {
    msg_id: MsgId,
    echo: String,
    in_reply_to: MsgId,
}

#[derive(Serialize, Deserialize, Debug)]
struct Init {
    msg_id: MsgId,
    node_id: String,
    node_ids: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct ErrorMsg {
    code: i64,
    in_reply_to: MsgId,
    text: String,
}

impl Echo {
    fn response(&self, new_msg_id: MsgId) -> ResponseBody {
        ResponseBody::EchoResponse(EchoResponse {
            msg_id: new_msg_id,
            echo: self.echo.clone(),
            in_reply_to: self.msg_id,
        })
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct InitResponse {
    msg_id: MsgId,
    in_reply_to: MsgId,
}

impl Init {
    fn response(&self, new_msg_id: MsgId) -> ResponseBody {
        ResponseBody::InitResponse(InitResponse {
            msg_id: new_msg_id,
            in_reply_to: self.msg_id,
        })
    }
}
