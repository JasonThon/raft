use std::error::Error;

use tokio::io::AsyncReadExt;
use tokio::sync::mpsc;

use crate::message::Message;

pub struct Peer {
    id: u32,
    url: String,
    peers_url: Vec<String>,
}

impl Peer {
    pub(crate) fn id(&self) -> u32 {
        self.id.clone()
    }

    pub(crate) fn send_to(&self, msg: Message) -> tokio::io::Result<()> {
        todo!()
    }
}

pub(crate) enum NetPlan<T> {
    Listener(usize, mpsc::Sender<T>),
    BroadCaster(Vec<String>),
    Sender(String),
}

impl<'a, T> NetPlan<T> where T: serde::Deserialize<'a> {
    pub(crate) async fn listen(&self) -> tokio::io::Result<()> {
        match self {
            NetPlan::Listener(port, msg_sender_chan) => {
                let listener = tokio::net::TcpListener::bind(
                    format!("localhost:{}", port))
                    .await?;

                loop {
                    listener.accept().await
                        .map(|(mut stream, _)| {
                            let ref mut buf = Vec::new();
                            stream.read_to_end(buf);

                            buf
                        })
                        .and_then(|buf|
                            match bincode::deserialize::<T>(buf.as_slice()) {
                                Ok(msg) => Ok(msg),
                                Err(err) => Err(
                                    tokio::io::Error::new(
                                        tokio::io::ErrorKind::Other,
                                        err.to_string(),
                                    )
                                )
                            })
                        .and_then(|result| msg_sender_chan.send(result))
                }
            }
            NetPlan::Sender(_) => {
                panic!("Listen is Unsupported for Sender net plan")
            }
            NetPlan::BroadCaster(_) => {
                panic!("Listen is Unsupported for Sender net plan")
            }
        }
    }
}