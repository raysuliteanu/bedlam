use std::io::{StdoutLock, Write};

use anyhow::Context;
use serde::Serialize;
use serde_json::Serializer;

use bedlam::*;

fn main() -> anyhow::Result<()> {
    let stdin = std::io::stdin().lock();
    let mut stdout = std::io::stdout().lock();

    let input_stream = serde_json::Deserializer::from_reader(stdin).into_iter::<Mesg>();
    // let mut output: Serializer<StdoutLock> = serde_json::Serializer::new(stdout);

    // let node = if let Ok(mesg) = input_stream.next().context("failed to read input stream")? {
    //     match mesg.body.payload {
    //         Payload::Init(node) => {
    //             let node = Node {
    //                 node_id: node.node_id,
    //                 node_ids: node.node_ids,
    //                 guid: 1,
    //             };
    //
    //             let init_ok = Mesg {
    //                 src: node.node_id.clone(),
    //                 dst: mesg.src,
    //                 body: Body {
    //                     msg_ty: "init_ok".to_string(),
    //                     msg_id: Some(0),
    //                     in_reply_to: mesg.body.msg_id,
    //                     payload: Payload::InitOk,
    //                 },
    //             };
    //
    //             init_ok
    //                 .serialize(&mut output)
    //                 .context("serialize init_ok response")?;
    //
    //             node
    //         }
    //         _ => {
    //             println!("{:?}", mesg);
    //             panic!("expected init message");
    //         }
    //     }
    // } else {
    //     panic!("init required")
    // };

    let mut this_node = Node {
        node_id: "uninit".to_string(),
        node_ids: vec![],
        guid: 0,
    };

    for input in input_stream {
        let mesg = input.context("message deserialization failed")?;
        match mesg.body.payload {
            Payload::Init(node) => {
                this_node = Node {
                    node_id: node.node_id,
                    node_ids: node.node_ids,
                    guid: 1,
                };

                let init_ok = Mesg {
                    src: this_node.node_id.clone(),
                    dst: mesg.src,
                    body: Body {
                        msg_id: Some(0),
                        in_reply_to: mesg.body.msg_id,
                        payload: Payload::InitOk,
                    },
                };

                let resp = serde_json::to_string(&init_ok).context("serialize init_ok response")?;
                writeln!(&mut stdout, "{resp}")?;

                // init_ok
                //     .serialize(&mut output)
                //     .context("serialize init_ok response")?;

                // node
            }
            Payload::InitOk => todo!("should not get an init_ok"),
            Payload::Echo(echo) => {
                let resp = Mesg {
                    src: this_node.node_id.clone(),
                    dst: mesg.src,
                    body: Body {
                        msg_id: Some(0),
                        in_reply_to: mesg.body.msg_id,
                        payload: Payload::EchoOk(Echo { echo: echo.echo }),
                    },
                };

                let resp = serde_json::to_string(&resp).context("serialize init_ok response")?;
                writeln!(&mut stdout, "{resp}")?;
                // resp.serialize(&mut output)
                //     .context("serialize init_ok response")?;
            }
            Payload::EchoOk(_) => {}
        }
    }

    Ok(())
}
