#![feature(test)]

use std::marker::Unpin;

use base64::encode;
use futures::sink::SinkExt;
use sha1::Sha1;
use tokio::io::{ AsyncRead, AsyncWrite, AsyncWriteExt };
use tokio::io::{ split, ReadHalf, WriteHalf };
use tokio::stream::StreamExt;
use tokio::sync::mpsc;
use tokio_util::codec::{ FramedRead, FramedWrite };

pub mod error;
pub mod codec;
pub mod message;
pub mod frame;

pub use error::{ WebsocketError, WebsocketResult };
pub use codec::WebsocketCodec;
pub use message::Message;
use frame::{
    Frame,
    FrameBuilder,
    Opcode,
};

#[derive(Debug)]
pub struct Websocket<S> {
    pub tx: mpsc::Sender<Message>,

    reader: FramedRead<ReadHalf<S>, WebsocketCodec>,
    key: Option<String>,
}

impl<S: 'static> Websocket<S>
where
    S: AsyncRead + AsyncWrite + Unpin + Send,
{
    /// Create a new websocket instance, given any type that implements
    /// `AsyncRead + AsyncWrite` like `tokio::net::TcpStream` or `tokio_native_tls::TlsStream`
    pub fn new(stream: S) -> Self {
        Self::_create(stream, None)
    }

    /// Same as `Websocket::new` except that it also accept a key that represent
    /// the value of `Sec-Websocket-Key` for client that requires a valid
    /// `Sec-Websocket-Accept` in response headers.
    pub fn new_with_key(stream: S, key: String) -> Self {
        Self::_create(stream, Some(key))
    }

    fn _create(stream: S, key: Option<String>) -> Self {
        let (reader, mut writer) = split(stream);
        let reader = FramedRead::new(reader, WebsocketCodec::default());
        let (tx, mut rx) = mpsc::channel::<Message>(100);

        tokio::spawn(async move {
            Self::_send_handshake(&mut writer, key).await;

            let mut writer = FramedWrite::new(writer, WebsocketCodec::default());

            while let Some(msg) = rx.recv().await {
                let is_close = msg.is_close;
                writer.send(msg).await.unwrap();
                if is_close { break }
            }
        });

        Self { reader, tx, key: None }
    }

    /// Send handshake.
    async fn _send_handshake(writer: &mut WriteHalf<S>, key: Option<String>) {
        let mut handshake = vec![
            "HTTP/1.1 101 Switching Protocols".to_string(),
            "Upgrade: websocket".to_string(),
            "Connection: Upgrade".to_string(),
        ];

        if let Some(key) = key {
            let guid = [key.as_bytes(), b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11"].concat();
            let sha1 = Sha1::from(guid).digest().bytes();
            let key = format!("Sec-Websocket-Accept: {}", encode(sha1));

            handshake.push(key);
        }

        handshake.push("\r\n".to_string());

        writer.write_all(handshake.join("\r\n").as_bytes()).await.unwrap();
    }

    /// Wait for next frame to come. (support incoming fragmented frames)
    pub async fn next(&mut self) -> Option<Message> {
        let mut frames = vec![];

        while let Some(Ok(frame)) = self.reader.next().await {
            let is_last = frame.is_last();

            if frame.is_control() {
                if frame.opcode.is_close() {
                    self.tx.send(Message::from_close(frame)).await.unwrap();
                    break
                } if frame.opcode.is_ping() {
                    self.tx.send(Message::create_pong_from_ping_frame(frame)).await.unwrap();
                }
            } else {
                frames.push(frame);

                if is_last {
                    return Some(Message::from_non_control_frames(frames))
                }
            }
        }

        None
    }
}
