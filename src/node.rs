use std::{
    collections::{HashMap, HashSet},
    io::{StdoutLock, Write},
    sync::mpsc::Receiver,
};

use anyhow::Context;

use crate::messages::{Body, Event, ExternalPayload, InternalPayload, Message};

#[allow(dead_code)]
pub struct Node<'n> {
    node_id: String,
    cluster: Vec<String>,
    topology: HashMap<String, Vec<String>>,
    uniq_msg_id: usize,
    // ids received from clients in `Broadcast` messages;
    // gossip them to nodes in our topology
    broadcast_ids: HashSet<i32>,
    // track messages sent to us via gossip from other nodes, so we don't send back
    // the same messages to them when we gossip to them
    known_ids: HashMap<String, HashSet<i32>>,
    input_stream: Receiver<Event>,
    output: StdoutLock<'n>,
}

impl<'n> Node<'n> {
    pub fn new(input: Receiver<Event>, output: StdoutLock<'n>) -> Self {
        Node {
            node_id: String::from(""),
            cluster: Vec::new(),
            topology: HashMap::new(),
            broadcast_ids: HashSet::new(),
            uniq_msg_id: 0,
            known_ids: HashMap::new(),
            input_stream: input,
            output,
        }
    }

    pub fn initialize(mut self) -> anyhow::Result<Self> {
        let event = self.input_stream.recv()?;
        if let Event::External { message } = event
            && let ExternalPayload::Init(init) = message.body.payload
        {
            let msg_id = Some(self.uniq_msg_id);
            self.uniq_msg_id += 1;
            let msg = serde_json::to_string(&Message {
                src: init.node_id.clone(),
                dst: message.src.clone(),
                body: Body {
                    msg_id,
                    in_reply_to: message.body.msg_id,
                    payload: &ExternalPayload::InitOk,
                },
            })?;
            eprintln!("sending message to {}: {}", message.src, msg);
            writeln!(self.output, "{}", msg).context("serialization failed")?;

            let node_id = init.node_id.to_string();
            let cluster = init.node_ids.to_vec();
            // initialize "default" topology to be the full cluster, until we
            // get a 'topology' message
            let mut topology: HashMap<String, Vec<String>> = HashMap::new();
            topology.insert(node_id.clone(), cluster.clone());

            let mut known_ids = HashMap::new();
            known_ids.insert(node_id.clone(), HashSet::new());

            Ok(Node {
                node_id,
                cluster,
                topology,
                known_ids,
                ..self
            })
        } else {
            panic!("expected init message")
        }
    }

    pub fn run(&mut self) -> anyhow::Result<()> {
        loop {
            match self.input_stream.recv()? {
                Event::Internal { payload } => match payload {
                    InternalPayload::Timer => {
                        eprintln!("received timer wakeup");
                        if !self.broadcast_ids.is_empty() {
                            let gossip_targets: Vec<(String, Vec<i32>)> = self
                                .topology
                                .get(&self.node_id)
                                .expect("always have our own topology node")
                                .iter()
                                .filter_map(|dest| {
                                    let messages: Vec<i32> = self
                                        .broadcast_ids
                                        .iter()
                                        .filter(|v| {
                                            let known = self
                                                .known_ids
                                                .get(dest)
                                                .expect("always have a known_for bucket");
                                            !known.contains(v)
                                        })
                                        .copied()
                                        .collect();
                                    if messages.is_empty() {
                                        None
                                    } else {
                                        Some((dest.clone(), messages))
                                    }
                                })
                                .collect();

                            for (dest, messages) in gossip_targets {
                                eprintln!("gossiping to {dest}");
                                let gossip = ExternalPayload::Gossip { messages };
                                self.send_to(&dest, None, &gossip)?;
                            }
                        }
                    }
                    InternalPayload::Eof => {
                        eprintln!("received EOF");
                        break;
                    }
                },
                Event::External { message } => match message.body.payload {
                    ExternalPayload::Init(_init) => panic!("got `init` but already initialized"),
                    ExternalPayload::Echo(echo) => {
                        self.send_to(
                            &message.src,
                            message.body.msg_id,
                            &ExternalPayload::EchoOk(echo),
                        )?;
                    }
                    ExternalPayload::Generate => {
                        self.send_to(
                            &message.src,
                            message.body.msg_id,
                            &ExternalPayload::GenerateOk {
                                id: format!("{}-{}", self.node_id, self.uniq_msg_id),
                            },
                        )?;
                    }
                    ExternalPayload::Broadcast { value } => {
                        self.send_to(
                            &message.src,
                            message.body.msg_id,
                            &ExternalPayload::BroadcastOk,
                        )?;
                        self.broadcast_ids.insert(value);
                    }
                    ExternalPayload::Topology { topology } => {
                        self.topology = topology;
                        self.known_ids.clear();
                        self.topology
                            .keys()
                            .filter(|n| **n != self.node_id)
                            .for_each(|k| {
                                self.known_ids.insert(k.clone(), HashSet::new());
                            });
                        eprintln!("new topology: {:?}", self.topology);
                        self.send_to(
                            &message.src,
                            message.body.msg_id,
                            &ExternalPayload::TopologyOk,
                        )?;
                    }
                    ExternalPayload::Read => self.send_to(
                        &message.src,
                        message.body.msg_id,
                        &ExternalPayload::ReadOk {
                            messages: self.broadcast_ids.iter().copied().collect(),
                        },
                    )?,
                    ExternalPayload::Gossip { messages } => {
                        eprintln!("received gossip from {}: {:?}", message.src, messages);
                        self.broadcast_ids.extend(&messages);
                        self.known_ids
                            .get_mut(&message.src)
                            .expect("always have an entry for node topology")
                            .extend(messages);
                        eprintln!(
                            "known ids for {}: {:?}",
                            message.src, self.known_ids[&message.src]
                        );
                    }
                    // ignore these ...
                    ExternalPayload::ReadOk { .. }
                    | ExternalPayload::InitOk
                    | ExternalPayload::EchoOk(_)
                    | ExternalPayload::GenerateOk { .. }
                    | ExternalPayload::BroadcastOk
                    | ExternalPayload::TopologyOk => {}
                },
            }
        }
        eprintln!("finished");
        Ok(())
    }

    fn send_to(
        &mut self,
        dst: &str,
        in_reply_to: Option<usize>,
        payload: &ExternalPayload,
    ) -> anyhow::Result<()> {
        let msg_id = Some(self.uniq_msg_id);
        self.uniq_msg_id += 1;
        let msg = serde_json::to_string(&Message {
            src: self.node_id.clone(),
            dst: dst.to_string(),
            body: Body {
                msg_id,
                in_reply_to,
                payload,
            },
        })?;
        eprintln!("sending message to {dst}: {msg}");
        writeln!(self.output, "{}", msg).context("serialization failed")
    }
}
