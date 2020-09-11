//! REFERENCE
//! =========
//! [RFC 6455 5.2](https://tools.ietf.org/html/rfc6455#section-5.2)
//!
//!  ```ignore
//!  0                   1                   2                   3
//!  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
//! +-+-+-+-+-------+-+-------------+-------------------------------+
//! |F|R|R|R| opcode|M| Payload len |    Extended payload length    |
//! |I|S|S|S|  (4)  |A|     (7)     |             (16/64)           |
//! |N|V|V|V|       |S|             |   (if payload len==126/127)   |
//! | |1|2|3|       |K|             |                               |
//! +-+-+-+-+-------+-+-------------+ - - - - - - - - - - - - - - - +
//! |     Extended payload length continued, if payload len == 127  |
//! + - - - - - - - - - - - - - - - +-------------------------------+
//! |                               |Masking-key, if MASK set to 1  |
//! +-------------------------------+-------------------------------+
//! | Masking-key (continued)       |          Payload Data         |
//! +-------------------------------- - - - - - - - - - - - - - - - +
//! :                     Payload Data continued ...                :
//! + - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - +
//! |                     Payload Data continued ...                |
//! +---------------------------------------------------------------+
//! ```
//!
//! ```ignore
//! FIN (1 bit): if 1 -> final fragment
//! RSV1 RSV2 RSV3 (3 bit): Must be 0 unless an extension defines them (SKIP)
//! Opcode (4 bit): Always assume its good (data sent from browser)
//!   - 0   -> continuation frame
//!   - 1   -> text frame
//!   - 2   -> binary frame
//!   - 3-7 -> reserved for further non-control frames
//!   - 8   -> connection closed
//!   - 9   -> ping
//!   - A   -> pong
//!   - B-F -> reserved for further control frames
//! Mask (1 bit): if 1 -> masking key is present. #section 5.3
//! Payload length (7 bit, 7+16 bit, 7+64 bit): length of the payload data
//!   - 0-125 -> thats the payload length
//!   - 126 -> next 2 bytes (UNSIGNED) are the payload length
//!   - 127 -> next 8 bytes (UNSIGNED) are the payload length
//! Masking key (0 or 4 bytes): present if mask bit is set to 1. #section 5.3
//! Payload data (x+y bytes): extension data + application data
//! Extension data (x bytes): is 0 unless an extension is negotiated (not in our case)
//! Application data (y bytes): is payload length - length of extension data
//! ```
use bytes::{ BytesMut, BufMut, Buf };

use std::convert::TryInto;

use crate::{ Frame, Opcode };

#[derive(Debug, Default)]
pub struct Builder {
    pub frame_index: usize,     // index of the frame in a fragmented message.

    first_byte_readed: bool,
    fin: bool,                  // 1 bit
    rsv: u8,                    // 3 bits
    opcode: Opcode,             // 4 bits

    second_byte_readed: bool,
    mask: bool,                 // 1 bit
    payload_len: u8,            // 0..=125,126 or 127

    masking_key_readed: bool,
    masking_key: [u8; 4],       // 0 or 4 bytes

    buf_len_readed: bool,
    buf_len: usize,             // expected payload length
    buf: BytesMut,              // received data
}

impl Builder {
    pub fn soft_reset(&mut self) {
        self.first_byte_readed = false;
        self.fin = false;
        self.rsv = 0;
        self.opcode = Opcode::Unset;
        self.second_byte_readed = false;
        self.mask = false;
        self.payload_len = 0;
        self.masking_key_readed = false;
        self.masking_key = [0, 0, 0, 0];
        self.buf_len_readed = false;
        self.buf = BytesMut::default();
    }

    pub fn build(&mut self, buf: &mut BytesMut) -> Option<Frame> {
        if !self.first_byte_readed {
            let byte = buf.get_u8();
            self._read_fin(byte);
            self._read_rsv(byte);

            if self.rsv != 0 {
                return Some(Frame::create_close_with_code(1002))
            }

            self._read_opcode(byte);

            if (matches!(self.opcode, Opcode::RsvControl | Opcode::RsvNonControl))
                || (self.frame_index == 0 && self.opcode == Opcode::Continuation)
                || (self.frame_index > 0 && matches!(self.opcode, Opcode::Text | Opcode::Binary))
            {
                return Some(Frame::create_close_with_code(1002))
            }

            self.first_byte_readed = true;
        }

        if !self.second_byte_readed {
            if buf.is_empty() { return None }
            let byte = buf.get_u8();
            self._read_mask(byte);
            self._read_payload_len(byte);
            self.second_byte_readed = true;

            if self.opcode.is_control() && (self.payload_len > 125 || self.fin == false) {
                return Some(Frame::create_close_with_code(1002))
            }
        }

        if !self.buf_len_readed {
            self._read_buf_len(buf)?;
            self.buf_len_readed = true;
        }

        if self.mask && !self.masking_key_readed {
            self._read_masking_key(buf)?;
            self.masking_key_readed = true;
        }

        if !self._read_buf(buf)? {
            //panic!("Invalid buffer");
        }

        let frame = Frame {
            fin: self.fin,
            rsv: self.rsv,
            opcode: self.opcode,
            mask: self.mask,
            payload_len: self.payload_len,
            buf_len: self.buf_len,
            masking_key: self.masking_key,
            buf: self.buf.split(),
        };

        Some(frame)
    }

    #[inline(always)]
    fn _read_buf(&mut self, buf: &mut BytesMut) -> Option<bool> {
        if !self.mask { panic!("masking key is not set!") }

        let idx = self.buf.len();

        for i in idx..(idx+buf.len()) {
            buf[i-idx] ^= self.masking_key[i % 4];
        }

        self.buf.put(buf);

        if self.buf.len() != self.buf_len {
            return None
        }

        Some(true)
    }

    #[inline(always)]
    fn _read_masking_key(&mut self, buf: &mut BytesMut) -> Option<()> {
        if buf.len() < 4 { return None }
        self.masking_key = buf[0..4].try_into().unwrap();
        buf.advance(4);

        Some(())
    }

    #[inline(always)]
    fn _read_buf_len(&mut self, buf: &mut BytesMut) -> Option<()> {
        match self.payload_len {
            0..=125 => self.buf_len = self.payload_len as usize,
            126 => {
                if buf.len() < 2 { return None }
                self.buf_len = buf.get_u16() as usize;
            },
            127 => {
                if buf.len() < 8 { return None }
                self.buf_len = buf.get_u64() as usize;
            },
            len => { panic!("Unsupported payload len value: {}", len) },
        }

        Some(())
    }

    #[inline(always)]
    fn _read_payload_len(&mut self, byte: u8) {
        self.payload_len = 0x7F & byte;
    }

    #[inline(always)]
    fn _read_mask(&mut self, byte: u8) {
        self.mask = (0x80 & byte) >> 7 != 0;
    }

    #[inline(always)]
    fn _read_fin(&mut self, byte: u8) {
        self.fin = (0x80 & byte) >> 7 != 0;
    }

    #[inline(always)]
    fn _read_rsv(&mut self, byte: u8) {
        self.rsv = (0x70 & byte) >> 4;
    }

    #[inline(always)]
    fn _read_opcode(&mut self, byte: u8) {
        match 0x0F & byte {
            0x0 => self.opcode = Opcode::Continuation,
            0x1 => self.opcode = Opcode::Text,
            0x2 => self.opcode = Opcode::Binary,
            0x3..=0x7 => self.opcode = Opcode::RsvNonControl,
            0x8 => self.opcode = Opcode::Close,
            0x9 => self.opcode = Opcode::Ping,
            0xA => self.opcode = Opcode::Pong,
            0xB..=0xF => self.opcode = Opcode::RsvControl,
            opcode => panic!("Unknown opcode is received: {}", opcode),
        }
    }
}
