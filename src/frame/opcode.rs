#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Opcode {
    Continuation,   // 0x0
    Text,           // 0x1
    Binary,         // 0x2
    RsvNonControl,  // 0x3..=0x7
    Close,          // 0x8
    Ping,           // 0x9
    Pong,           // 0xA
    RsvControl,     // 0xB..=0xF

    // denote that the opcode field is not set yet
    Unset,
}

impl Default for Opcode {
    fn default() -> Self { Opcode::Unset }
}

impl Opcode {
    pub fn is_continuation(&self) -> bool { *self == Opcode::Continuation }
    pub fn is_text(&self) -> bool { *self == Opcode::Text }
    pub fn is_binary(&self) -> bool { *self == Opcode::Binary }
    pub fn is_rsv_non_control(&self) -> bool { *self == Opcode::RsvNonControl }
    pub fn is_close(&self) -> bool { *self == Opcode::Close }
    pub fn is_ping(&self) -> bool { *self == Opcode::Ping }
    pub fn is_pong(&self) -> bool { *self == Opcode::Pong }
    pub fn is_rsv_control(&self) -> bool { *self == Opcode::RsvControl }

    pub fn is_control(&self) -> bool {
        matches!(*self,
            Opcode::Ping
            | Opcode::Pong
            | Opcode::Close
            | Opcode::RsvControl
        )
    }

    pub fn is_unset(&self) -> bool { *self == Opcode::Unset }
}
