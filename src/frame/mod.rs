use bytes::{ BytesMut, BufMut };

pub mod builder;
pub mod opcode;

pub use builder::Builder as FrameBuilder;
pub use opcode::Opcode;

#[derive(Debug, Default)]
pub struct Frame {
    pub fin: bool,              // 1 bit
    pub rsv: u8,                // 3 bits
    pub opcode: Opcode,         // 4 bits
    pub mask: bool,             // 1 bit
    pub payload_len: u8,        // <=125,126 or 127
    pub buf_len: usize,         // expected payload length
    pub masking_key: [u8; 4],       // 0 or 4 bytes
    pub buf: BytesMut,          // received data
}

impl Frame {
    pub fn is_last(&self) -> bool {
        self.fin
    }

    pub fn is_control(&self) -> bool {
        self.opcode.is_control()
    }

    pub fn is_non_control(&self) -> bool {
        !self.is_control()
    }

    pub fn create_close_with_code(code: u16) -> Self {
        let mut buf = BytesMut::new();
        buf.put_u16(code);

        Self {
            fin: true,
            rsv: 0,
            opcode: Opcode::Close,
            mask: false,
            payload_len: buf.len() as u8,
            buf_len: buf.len(),
            masking_key: [0, 0, 0, 0],
            buf,
        }
    }
}
