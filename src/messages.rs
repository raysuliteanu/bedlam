use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[allow(dead_code)]
pub enum Event {
    Internal { payload: InternalPayload },
    External { message: Message<ExternalPayload> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message<Payload> {
    pub src: String,
    #[serde(rename = "dest")]
    pub dst: String,
    pub body: Body<Payload>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Body<Payload> {
    pub msg_id: Option<usize>,
    pub in_reply_to: Option<usize>,
    #[serde(flatten)]
    pub payload: Payload,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum ExternalPayload {
    Init(Init),
    InitOk,
    Echo(Echo),
    EchoOk(Echo),
    Generate,
    GenerateOk {
        id: String,
    },
    Broadcast {
        #[serde(rename = "message")]
        value: i32,
    },
    BroadcastOk,
    Read,
    ReadOk {
        messages: Vec<i32>,
    },
    Topology {
        topology: HashMap<String, Vec<String>>,
    },
    TopologyOk,
    Gossip {
        messages: Vec<i32>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Init {
    pub node_id: String,
    pub node_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Echo {
    pub echo: String,
}

#[allow(dead_code)]
pub enum InternalPayload {
    Timer,
    Eof,
}
