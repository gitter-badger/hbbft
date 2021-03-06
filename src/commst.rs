//! Comms task structure. A comms task communicates with a remote node through a
//! socket. Local communication with coordinating threads is made via
//! `crossbeam_channel::unbounded()`.
use crossbeam;
use crossbeam_channel::{Receiver, Sender};
use std::fmt::Debug;
use std::io;
use std::net::TcpStream;
use std::sync::Arc;

use messaging::SourcedMessage;
use proto::Message;
use proto_io;
use proto_io::ProtoIo;

#[derive(Debug)]
pub enum Error {
    IoError(io::Error),
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error {
        Error::IoError(err)
    }
}

/// A communication task connects a remote node to the thread that manages the
/// consensus algorithm.
pub struct CommsTask<'a, T: 'a + Clone + Debug + Send + Sync + From<Vec<u8>> + Into<Vec<u8>>> {
    /// The transmit side of the multiple producer channel from comms threads.
    tx: &'a Sender<SourcedMessage<T>>,
    /// The receive side of the channel to the comms thread.
    rx: &'a Receiver<Message<T>>,
    /// The socket IO task.
    io: ProtoIo,
    /// The index of this comms task for identification against its remote node.
    pub node_index: usize,
}

impl<'a, T: 'a + Clone + Debug + Send + Sync + From<Vec<u8>> + Into<Vec<u8>>> CommsTask<'a, T> {
    pub fn new(
        tx: &'a Sender<SourcedMessage<T>>,
        rx: &'a Receiver<Message<T>>,
        stream: TcpStream,
        node_index: usize,
    ) -> Self {
        debug!(
            "Creating comms task #{} for {:?}",
            node_index,
            stream.peer_addr().unwrap()
        );

        CommsTask {
            tx,
            rx,
            io: ProtoIo::from_stream(stream),
            node_index,
        }
    }

    /// The main socket IO loop and an asynchronous thread responding to manager
    /// thread requests.
    pub fn run(&mut self) -> Result<(), Error> {
        // Borrow parts of `self` before entering the thread binding scope.
        let tx = Arc::new(self.tx);
        let rx = Arc::new(self.rx);
        let mut io1 = self.io.try_clone()?;
        let node_index = self.node_index;

        crossbeam::scope(|scope| {
            // Local comms receive loop thread.
            scope.spawn(move || {
                loop {
                    // Receive a multicast message from the manager thread.
                    let message = rx.recv().unwrap();
                    debug!("Node {} <- {:?}", node_index, message);
                    // Forward the message to the remote node.
                    io1.send(message).unwrap();
                }
            });

            // Remote comms receive loop.
            debug!("Starting remote RX loop for node {}", node_index);
            loop {
                match self.io.recv() {
                    Ok(message) => {
                        debug!("Node {} -> {:?}", node_index, message);
                        tx.send(SourcedMessage {
                            source: node_index,
                            message,
                        }).unwrap();
                    }
                    Err(proto_io::Error::ProtobufError(e)) => {
                        warn!("Node {} - Protobuf error {}", node_index, e)
                    }
                    Err(e) => {
                        warn!("Node {} - Critical error {:?}", node_index, e);
                        break;
                    }
                }
            }
        });
        Ok(())
    }
}
