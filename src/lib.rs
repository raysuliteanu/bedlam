use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mesg {
    pub src: String,
    #[serde(rename = "dest")]
    pub dst: String,
    pub body: Body,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Body {
    pub msg_id: Option<usize>,
    pub in_reply_to: Option<usize>,
    #[serde(flatten)]
    pub payload: Payload,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum Payload {
    Init(Init),
    InitOk,
    Echo(Echo),
    EchoOk(Echo),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Init {
    pub node_id: String,
    // node_ids includes node_id
    pub node_ids: Vec<String>,
}

pub struct Node {
    pub node_id: String,
    // node_ids includes node_id
    pub node_ids: Vec<String>,
    pub guid: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Error {
    code: u32,
    text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Echo {
    pub echo: String,
}
