use bytes::{ Bytes, BytesMut, BufMut };

use crate::{ Frame, Opcode };

#[derive(Debug, Default)]
pub struct Message {
    pub is_text: bool,
    pub is_binary: bool,
    pub is_close: bool,
    pub is_pong: bool,
    pub buf: Bytes,

    frames: Vec<Frame>,
}

impl Message {
    pub fn new() -> Self {
        Self { ..Default::default() }
    }

    pub fn create_pong_from_ping_frame(frame: Frame) -> Self {
        assert!(frame.opcode.is_ping());

        Self {
            is_pong: true,
            buf: frame.buf.freeze(),
            ..Default::default()
        }
    }

    pub fn from_non_control_frames(frames: Vec<Frame>) -> Self {
        assert!(frames.len() > 0);

        Self {
            is_text: frames[0].opcode.is_text(),
            is_binary: frames[0].opcode.is_binary(),
            frames,
            ..Default::default()
        }
    }

    pub fn text(&self) -> String {
        assert!(self.is_text);

        String::from("hello")
    }

    pub fn binary(&self) -> Bytes {
        assert!(self.is_binary);

        let mut buf = BytesMut::new();

        for frame in self.frames.iter() {
            buf.put_slice(&frame.buf[..]);
        }

        buf.freeze()
    }

    pub fn from_close(frame: Frame) -> Self {
        Self {
            is_close: true,
            buf: frame.buf.freeze(),
            ..Default::default()
        }
    }

    pub fn from_text(text: String) -> Self {
        Self {
            is_text: true,
            buf: Bytes::from(text),
            ..Default::default()
        }
    }

    pub fn from_binary(buf: Bytes) -> Self {
        Self {
            is_binary: true,
            buf,
            ..Default::default()
        }
    }
}
