use std::convert::TryInto;

use bytes::{ Bytes, BytesMut, BufMut };
use tokio_util::codec::{ Decoder, Encoder };

use crate::{
    Message,
    WebsocketError,
    WebsocketResult,
    Frame,
    FrameBuilder,
    Opcode,
};

#[derive(Debug, Default)]
pub struct WebsocketCodec {
    frame_builder: FrameBuilder,
}

impl Decoder for WebsocketCodec {
    type Item = Frame;
    type Error = WebsocketError;

    fn decode(&mut self, buf: &mut BytesMut) -> WebsocketResult<Option<Self::Item>> {
        if buf.is_empty() {
            return Ok(None)
        }

        if let Some(frame) = self.frame_builder.build(buf) {
            let frame_index = self.frame_builder.frame_index;

            self.frame_builder.soft_reset();

            if (frame.fin == false && matches!(frame.opcode, Opcode::Text | Opcode::Binary | Opcode::Continuation))
                || (frame_index > 0 && matches!(frame.opcode, Opcode::Ping))
            {
                self.frame_builder.frame_index = frame_index + 1;
            } else if frame.fin == true && matches!(frame.opcode, Opcode::Continuation) {
                self.frame_builder.frame_index = 0;
            }

            return Ok(Some(frame))
        }

        Ok(None)
    }
}

impl Encoder<Message> for WebsocketCodec {
    type Error = WebsocketError;

    fn encode(&mut self, msg: Message, buf: &mut BytesMut) -> WebsocketResult<()> {
        if msg.is_close { buf.put_u8(0x88); /* 1000 1000 */ }
        else if msg.is_text { buf.put_u8(0x81); /* 1000 0001 */ }
        else if msg.is_binary { buf.put_u8(0x82); /* 1000 0010 */ }
        else if msg.is_pong { buf.put_u8(0x8A); /* 1000 1010 */ }
        else { unimplemented!() };

        if msg.buf.len() > u16::MAX as usize {
            buf.put_u8(0x7F); // 0111 1111
            buf.put_u64(msg.buf.len() as u64);
        } else if msg.buf.len() > 125 {
            buf.put_u8(0x7E); // 0111 1110
            buf.put_u16(msg.buf.len() as u16);
        } else {
            buf.put_u8(msg.buf.len() as u8);
        }

        buf.put(msg.buf);

        Ok(())
    }
}
