use std::{
    collections::VecDeque,
    io::{self, ErrorKind, Read, Write},
    net::TcpStream,
};

use serde::{de::DeserializeOwned, Serialize};

const HEADER_LEN: usize = 2;

#[derive(Debug)]
pub struct NetworkMessage {
    content_len: [u8; HEADER_LEN],
    buf: Vec<u8>,
}

pub fn serialize<M>(message: M) -> NetworkMessage
where
    M: Serialize,
{
    let buf = bincode::serialize(&message).expect("bincode failed to serialize message");
    let content_len = u16::try_from(buf.len())
        .expect("bincode message length overflowed")
        .to_le_bytes();
    NetworkMessage { content_len, buf }
}

pub fn send(
    network_message: &NetworkMessage,
    stream: &mut TcpStream,
) -> Result<(), io::Error> {
    // Prefix data by length so it's easy to parse on the other side.
    stream.write_all(&network_message.content_len)?;
    stream.write_all(&network_message.buf)?;
    stream.flush()?; // LATER No idea if necessary or how it interacts with set_nodelay

    Ok(())
}

/// Read bytes from `stream` into `buffer`,
/// parse messages that are complete and add them to `messages`.
///
/// Returns whether the connection has been closed (doesn't matter if cleanly or reading failed).
#[must_use]
pub fn receive<M>(
    stream: &mut TcpStream,
    buffer: &mut VecDeque<u8>,
    messages: &mut Vec<M>,
) -> bool
where
    M: DeserializeOwned,
{
    // Read all available bytes until the stream would block.
    let mut closed = false;
    loop {
        // No particular reason for the buffer size, except BufReader uses the same.
        let mut buf = [0; 8192];
        let res = stream.read(&mut buf);
        match res {
            Ok(0) => {
                // The connection has been closed, don't get stuck in this loop.
                //println!("Connection closed when reading");
                closed = true;
                break;
            }
            Ok(n) => {
                buffer.extend(&buf[0..n]);
            }
            Err(e) if e.kind() == ErrorKind::Interrupted => {}
            Err(e) if e.kind() == ErrorKind::WouldBlock => {
                break;
            }
            Err(e) => {
                println!("Connection closed when reading - error: {}", e);
                closed = true;
                break;
            }
        }
    }

    // Parse the received bytes
    loop {
        if buffer.len() < HEADER_LEN {
            break;
        }
        let len_bytes = [buffer[0], buffer[1]];
        let content_len = usize::from(u16::from_le_bytes(len_bytes));
        if buffer.len() < HEADER_LEN + content_len {
            // Not enough bytes in buffer for a full frame.
            break;
        }
        buffer.pop_front();
        buffer.pop_front();
        let bytes: Vec<_> = buffer.drain(0..content_len).collect();
        let message = bincode::deserialize(&bytes).unwrap();
        messages.push(message);
    }

    closed
}
