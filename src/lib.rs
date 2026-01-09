use anyhow::Context;
use serde_json::{StreamDeserializer, de::IoRead};
use std::io::{StdinLock, StdoutLock, Write};

use crate::messages::{Body, Echo, Mesg, Payload};

pub mod messages {
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

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Error {
        code: u32,
        text: Option<String>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Echo {
        pub echo: String,
    }
}

pub struct UninitializedNode<'a> {
    input: StdinLock<'a>,
    output: StdoutLock<'a>,
}

pub struct IntializedNode<'de> {
    pub node_id: String,
    // node_ids includes node_id
    pub node_ids: Vec<String>,
    pub guid: usize,
    input_stream: StreamDeserializer<'de, IoRead<StdinLock<'de>>, Mesg>,
    output: StdoutLock<'de>,
}

pub struct Node<S> {
    state: S,
}

impl<'a> Node<UninitializedNode<'a>> {
    pub fn new(input: StdinLock<'a>, output: StdoutLock<'a>) -> Self {
        Node {
            state: UninitializedNode { input, output },
        }
    }

    pub fn initialize(mut self) -> anyhow::Result<Node<IntializedNode<'a>>> {
        let mut input_stream =
            serde_json::Deserializer::from_reader(self.state.input).into_iter::<Mesg>();
        let mut guid: usize = 0;
        let init = if let Ok(mesg) = input_stream.next().context("failed to read input stream")? {
            match mesg.body.payload {
                Payload::Init(node) => {
                    let init_ok = Mesg {
                        src: node.node_id.clone(),
                        dst: mesg.src,
                        body: Body {
                            msg_id: Some(guid),
                            in_reply_to: mesg.body.msg_id,
                            payload: Payload::InitOk,
                        },
                    };
                    writeln!(
                        &mut self.state.output,
                        "{}",
                        serde_json::to_string(&init_ok).context("serialize init_ok response")?
                    )?;
                    guid += 1;
                    node
                }
                _ => {
                    eprintln!("{:?}", mesg);
                    panic!("expected init message");
                }
            }
        } else {
            todo!()
        };

        Ok(Node {
            state: IntializedNode {
                node_id: init.node_id,
                node_ids: init.node_ids,
                guid,
                input_stream,
                output: self.state.output,
            },
        })
    }
}

impl<'a> Node<IntializedNode<'a>> {
    pub fn process_messages(mut self) -> anyhow::Result<()> {
        for input in self.state.input_stream {
            let mesg: Mesg = input.context("message deserialization failed")?;
            match mesg.body.payload {
                Payload::Init(_) => todo!("should not get an init"),
                Payload::InitOk => todo!("should not get an init_ok"),
                Payload::Echo(echo) => {
                    let echo_ok = Mesg {
                        src: self.state.node_id.clone(),
                        dst: mesg.src,
                        body: Body {
                            msg_id: Some(self.state.guid),
                            in_reply_to: mesg.body.msg_id,
                            payload: Payload::EchoOk(Echo { echo: echo.echo }),
                        },
                    };

                    self.state.guid += 1;

                    writeln!(
                        &mut self.state.output,
                        "{}",
                        serde_json::to_string(&echo_ok).context("serialize init_ok response")?
                    )?;
                }
                Payload::EchoOk(_) => {}
            }
        }
        Ok(())
    }
}
