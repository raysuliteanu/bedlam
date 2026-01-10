use anyhow::Context;
use serde_json::{StreamDeserializer, de::IoRead};
use std::{
    collections::{HashMap, HashSet},
    io::{StdinLock, StdoutLock, Write},
};

use crate::messages::{Body, Echo, Mesg, Payload};

pub mod messages {
    use std::collections::HashMap;

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
    pub struct Init {
        pub node_id: String,
        // node_ids includes node_id
        pub node_ids: Vec<String>,
    }

    // The code is an integer which indicates the type of error which occurred.
    // Maelstrom defines several error types, and you can also invent your own.
    // Codes 0-999 are reserved for Maelstrom's use; codes 1000 and above are
    // free for your own purposes.
    // The text field is a free-form string. It is optional, and may contain any
    // explanatory message you like. You may include other keys in the error body,
    // if you like; Maelstrom will retain them as a part of the history, and they
    // may be helpful in your own analysis.
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Error {
        code: u32,
        text: Option<String>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "snake_case", tag = "type")]
    pub enum Payload {
        Init(Init),
        InitOk,
        Echo(Echo),
        EchoOk(Echo),
        Generate,
        GenerateOk {
            id: String,
        },
        Broadcast {
            message: i32,
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
    pub cluster: HashSet<String>,
    pub topology: HashMap<String, Vec<String>>,
    pub msg_id: usize,
    input_stream: StreamDeserializer<'de, IoRead<StdinLock<'de>>, Mesg>,
    output: StdoutLock<'de>,
    broadcast_ids: Vec<i32>,
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
        let mesg = input_stream.next().expect("expected an 'init' message")?;
        let init = match mesg.body.payload {
            Payload::Init(node) => {
                let init_ok = Mesg {
                    src: node.node_id.clone(),
                    dst: mesg.src,
                    body: Body {
                        msg_id: Some(0),
                        in_reply_to: mesg.body.msg_id,
                        payload: Payload::InitOk,
                    },
                };

                writeln!(
                    &mut self.state.output,
                    "{}",
                    serde_json::to_string(&init_ok).context("serialize init_ok response")?
                )?;

                node
            }
            _ => {
                panic!("expected init message, got {:?}", mesg);
            }
        };

        let mut cluster: HashSet<String> = HashSet::from_iter(init.node_ids);
        cluster.retain(|id| *id != init.node_id);
        assert!(!cluster.contains(&init.node_id));

        let mut topology: HashMap<String, Vec<String>> = HashMap::new();
        topology.insert(init.node_id.clone(), cluster.iter().cloned().collect());

        Ok(Node {
            state: IntializedNode {
                node_id: init.node_id,
                cluster,
                topology,
                msg_id: 1,
                input_stream,
                output: self.state.output,
                broadcast_ids: Vec::new(),
            },
        })
    }
}

impl<'a> Node<IntializedNode<'a>> {
    fn broadcast(&mut self, payload: &Payload) -> anyhow::Result<()> {
        let dests = &self
            .state
            .topology
            .get(&self.state.node_id)
            .expect("should always have some topo since initialized at init")
            .clone();

        eprintln!("dests: {dests:?}");
        dests
            .iter()
            .try_for_each(|dest| self.send(dest, None, payload))?;

        Ok(())
    }

    fn send(
        &mut self,
        dst: &str,
        in_reply_to_id: Option<usize>,
        payload: &Payload,
    ) -> anyhow::Result<()> {
        let mesg = Mesg {
            src: self.state.node_id.clone(),
            dst: dst.to_string(),
            body: Body {
                msg_id: Some(self.state.msg_id),
                in_reply_to: in_reply_to_id,
                payload: payload.clone(),
            },
        };

        self.state.msg_id += 1;

        writeln!(
            &mut self.state.output,
            "{}",
            serde_json::to_string(&mesg).context("serialize response")?
        )
        .context("serialize response failed")
    }

    pub fn process_messages(&mut self) -> anyhow::Result<()> {
        while let Some(input) = self.state.input_stream.next() {
            let mesg: Mesg = input.context("message deserialization failed")?;
            match mesg.body.payload {
                Payload::Init(_) => todo!("should not get an init"),
                Payload::InitOk => todo!("should not get an init_ok"),
                Payload::Echo(ref echo) => {
                    self.send(
                        &mesg.src,
                        mesg.body.msg_id,
                        &Payload::EchoOk(Echo {
                            echo: echo.echo.clone(),
                        }),
                    )?;
                }
                Payload::EchoOk(_) => {}
                Payload::Generate => {
                    let id = format!("{}-{}", self.state.node_id, self.state.msg_id);
                    self.send(&mesg.src, mesg.body.msg_id, &Payload::GenerateOk { id })?;
                }
                Payload::GenerateOk { .. } => {}
                Payload::Broadcast { message } => {
                    self.state.broadcast_ids.push(message);
                    self.broadcast(&Payload::Broadcast { message })?;
                    self.send(&mesg.src, mesg.body.msg_id, &Payload::BroadcastOk)?;
                }
                Payload::BroadcastOk => {}
                Payload::Read => {
                    self.send(
                        &mesg.src,
                        mesg.body.msg_id,
                        &Payload::ReadOk {
                            messages: self.state.broadcast_ids.clone(),
                        },
                    )?;
                }
                Payload::ReadOk { .. } => {}
                Payload::Topology { topology } => {
                    self.state.topology = topology;
                    self.send(&mesg.src, mesg.body.msg_id, &Payload::TopologyOk)?;
                }
                Payload::TopologyOk => {}
            }
        }
        Ok(())
    }
}
