use std::{sync::mpsc::Sender, thread::JoinHandle, time::Duration};

use crate::{
    messages::{Event, ExternalPayload, InternalPayload, Message},
    node::Node,
};

mod messages;
mod node;

fn main() -> anyhow::Result<()> {
    let (tx, rx) = std::sync::mpsc::channel::<Event>();

    let io_loop = start_io_loop(tx.clone());
    let timer = start_timer(tx);

    let stdout = std::io::stdout().lock();

    let node = Node::new(rx, stdout);
    node.initialize()?.run()?;

    eprintln!("cleaning up ...");
    io_loop.join().expect("join io_loop");
    timer.join().expect("join on timer");

    eprintln!("exiting normally");
    Ok(())
}

fn start_io_loop(tx: Sender<Event>) -> JoinHandle<()> {
    std::thread::spawn(move || {
        let mut buf = String::new();
        loop {
            let stdin = std::io::stdin();
            match stdin.read_line(&mut buf) {
                Ok(0) => {
                    eprintln!("sending EOF");
                    let _todo = tx.send(Event::Internal {
                        payload: InternalPayload::Eof,
                    });
                    break;
                }
                Ok(_bytes_read) => match serde_json::from_str::<Message<ExternalPayload>>(&buf) {
                    Ok(message) => {
                        let _todo = tx.send(Event::External { message });
                    }
                    Err(e) => panic!("deserialization failed: {e}"),
                },
                Err(e) => panic!("read error on stdin: {e}"),
            }
            buf.clear();
        }
        eprintln!("IO loop complete");
    })
}

fn start_timer(tx: Sender<Event>) -> JoinHandle<()> {
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(Duration::from_millis(250));
            if tx
                .send(Event::Internal {
                    payload: InternalPayload::Timer,
                })
                .is_err()
            {
                eprintln!("timer thread send failed; receiver probably exited; exiting ...");
                break;
            }
        }
    })
}
