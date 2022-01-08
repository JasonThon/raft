use std::io::Error;

use tokio::io::AsyncReadExt;
use tokio::sync::mpsc;

use crate::message::Message;
use crate::rafterror::RaftError;

#[derive(Clone)]
pub struct Peer {
    id: u64,
    url: String,
    peers_url: Vec<String>,
}

impl Peer {
    pub(crate) fn id(&self) -> u64 {
        self.id.clone()
    }

    pub(crate) fn send_to(&self, msg: Message, _id: u64) -> tokio::io::Result<()> {
        todo!()
    }

    pub(crate) fn broadcast(&self, msg: Message) -> Result<(), RaftError> {
        todo!()
    }

    pub(crate) fn new(id: u64, url:&str, peers_url: Vec<String>) -> Peer {
        Peer {
            id,
            url: String::from(url),
            peers_url
        }
    }
}

pub(crate) enum NetPlan {
    Listener(usize),
    BroadCaster(Vec<String>),
    Sender(String),
}

impl NetPlan {
    pub(crate) async fn listen_and_send(&self, msg_sender_chan: mpsc::UnboundedSender<Message>)
                                        -> tokio::io::Result<()> {
        match self {
            NetPlan::Listener(port) => {
                let listener = tokio::net::TcpListener::bind(
                    format!("localhost:{}", port))
                    .await?;

                loop {
                    listener.accept()
                        .await
                        .map(|(mut stream, _)| {
                            let ref mut buf = Vec::new();
                            stream.read_to_end(buf);

                            buf.to_vec()
                        })
                        .and_then(|buf| Self::deserialize(&buf))
                        .and_then(|result|
                            msg_sender_chan.send(result)
                                .map_err(|err|
                                    tokio::io::Error::new(
                                        tokio::io::ErrorKind::Other,
                                        format!("{}", err),
                                    )
                                )
                        )
                        .unwrap_or(())
                }
            }
            NetPlan::Sender(_) => {
                Err(tokio::io::Error::new(tokio::io::ErrorKind::ConnectionRefused, "Listen is Unsupported for Sender net plan"))
            }
            NetPlan::BroadCaster(_) => {
                Err(tokio::io::Error::new(tokio::io::ErrorKind::ConnectionRefused, "Listen is Unsupported for Sender net plan"))
            }
        }
    }

    fn deserialize(buf: &[u8]) -> Result<Message, Error> {
        match bincode::deserialize::<Message>(buf) {
            Ok(msg) => Ok(msg),
            Err(err) => Err(
                tokio::io::Error::new(
                    tokio::io::ErrorKind::Other,
                    err.to_string(),
                )
            )
        }
    }
}