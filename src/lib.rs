use ::log::{debug, info};
use anyhow::Context;
use serde_json::{StreamDeserializer, de::IoRead};
use std::{
    collections::{HashMap, HashSet},
    io::{StdinLock, StdoutLock, Write},
};

use crate::messages::{Body, Echo, Mesg, Payload};

pub mod log;

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

    impl std::fmt::Display for Mesg {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(
                f,
                "src: {}, dest: {}, body: {}",
                self.src, self.dst, self.body
            )
        }
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Body {
        pub msg_id: Option<usize>,
        pub in_reply_to: Option<usize>,
        #[serde(flatten)]
        pub payload: Payload,
    }

    impl std::fmt::Display for Body {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let msg_id = if let Some(msg_id) = self.msg_id {
                &usize::to_string(&msg_id)
            } else {
                "none"
            };
            let in_reply_to = if let Some(in_reply_to) = self.in_reply_to {
                &usize::to_string(&in_reply_to)
            } else {
                "none"
            };
            write!(
                f,
                "msg_id: {msg_id}, in_reply_to: {in_reply_to}, payload: {:?}",
                self.payload
            )
        }
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
    broadcast_ids: Vec<i32>,
    input_stream: StreamDeserializer<'de, IoRead<StdinLock<'de>>, Mesg>,
    output: StdoutLock<'de>,
}

impl std::fmt::Display for IntializedNode<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let cluster = self
            .cluster
            .iter()
            .cloned()
            .collect::<Vec<String>>()
            .join(",");

        let topology = self
            .topology
            .values()
            .flatten()
            .cloned()
            .collect::<Vec<String>>()
            .join(",");

        let broadcast_ids = self
            .broadcast_ids
            .iter()
            .take(10)
            .map(|v| (*v).to_string())
            .collect::<Vec<String>>()
            .join(",");

        write!(
            f,
            "node_id: {}, cluster: {}, topology: {}, msg_id: {}, broadcast_ids: [{}...]",
            self.node_id, cluster, topology, self.msg_id, broadcast_ids,
        )
    }
}

pub struct Node<S> {
    state: S,
}

impl<S: std::fmt::Display> std::fmt::Display for Node<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.state)
    }
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
                send_to(
                    &mut self.state.output,
                    &node.node_id,
                    &mesg.src,
                    0,
                    mesg.body.msg_id,
                    &Payload::InitOk,
                )?;

                node
            }
            _ => {
                panic!("expected init message, got {mesg}");
            }
        };

        let mut cluster: HashSet<String> = HashSet::from_iter(init.node_ids);
        cluster.retain(|id| *id != init.node_id);
        assert!(!cluster.contains(&init.node_id));

        let mut topology: HashMap<String, Vec<String>> = HashMap::new();
        topology.insert(init.node_id.clone(), cluster.iter().cloned().collect());

        let node = Node {
            state: IntializedNode {
                node_id: init.node_id,
                cluster,
                topology,
                msg_id: 1,
                input_stream,
                output: self.state.output,
                broadcast_ids: Vec::new(),
            },
        };

        info!("initialized node: {node}");

        Ok(node)
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

        debug!("broadcasting to dests: {dests:?}");
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
        send_to(
            &mut self.state.output,
            &self.state.node_id,
            dst,
            self.state.msg_id,
            in_reply_to_id,
            payload,
        )?;

        self.state.msg_id += 1;

        Ok(())
    }

    pub fn process_messages(&mut self) -> anyhow::Result<()> {
        while let Some(input) = self.state.input_stream.next() {
            let mesg: Mesg = input.context("message deserialization failed")?;
            debug!("processing mesg: {mesg}");
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

fn send_to(
    write: &mut impl Write,
    src: &str,
    dst: &str,
    msg_id: usize,
    in_reply_to_id: Option<usize>,
    payload: &Payload,
) -> anyhow::Result<()> {
    let mesg = Mesg {
        src: src.to_string(),
        dst: dst.to_string(),
        body: Body {
            msg_id: Some(msg_id),
            in_reply_to: in_reply_to_id,
            payload: payload.clone(),
        },
    };

    debug!("sending: {mesg}");

    writeln!(
        write,
        "{}",
        serde_json::to_string(&mesg).context("serialize response")?
    )
    .context("serialize response failed")
}
