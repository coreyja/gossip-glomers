use color_eyre::eyre::Result;
use serde::{Deserialize, Serialize};

use std::io::BufRead;

fn main() -> Result<()> {
    let stdin = std::io::stdin();

    loop {
        let mut buffer = String::new();
        let bytes = stdin.read_line(&mut buffer)?;

        if bytes != 0 && !buffer.is_empty() {
            eprintln!("Received: {}", buffer);
            // if m.body.t != "echo" {
            //     panic!("Unexpected message type: {:?}", m.body.t);
            // }

            let m = serde_json::from_str::<Message>(&buffer)?;

            let resp = match &m.body {
                Body::Echo(e) => Some(e.respond(m.src, m.dest)),
                Body::Init(e) => Some(e.respond(m.src, m.dest)),
                Body::InitResponse(_) => None,
                _ => panic!("Unexpected message type: {:?}", m.body),
            };

            if let Some(resp) = resp {
                let output = serde_json::to_string(&resp)?;

                println!("{output}");
            }
        }
    }
}

type MsgId = u64;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct Message {
    body: Body,
    dest: String,
    src: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
enum Body {
    #[serde(rename = "echo")]
    Echo(Echo),
    #[serde(rename = "echo_ok")]
    EchoResponse(EchoResponse),
    #[serde(rename = "init")]
    Init(Init),
    #[serde(rename = "error")]
    ErrorMsg(ErrorMsg),
    #[serde(rename = "init_ok")]
    InitResponse(InitResponse),
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct NotFullMessageBody {
    #[serde(rename = "type")]
    t: String,
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
    fn respond(&self, dest: String, src: String) -> Message {
        let resp = EchoResponse {
            msg_id: self.msg_id + 1,
            echo: self.echo.clone(),
            in_reply_to: self.msg_id,
        };

        Message {
            body: Body::EchoResponse(resp),
            dest,
            src,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct InitResponse {
    msg_id: MsgId,
    in_reply_to: MsgId,
}

impl Init {
    fn respond(&self, dest: String, src: String) -> Message {
        let resp = InitResponse {
            msg_id: self.msg_id + 1,
            in_reply_to: self.msg_id,
        };

        Message {
            body: Body::InitResponse(resp),
            dest,
            src,
        }
    }
}
