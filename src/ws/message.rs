use std::io::net::tcp::TcpStream;
use std::io::{IoResult, IoError};
use std::io::IoErrorKind::InvalidInput;
use std::num;

pub use self::Payload::{Text, Binary, Empty};
pub use self::Opcode::{ContinuationOp, TextOp, BinaryOp, CloseOp, PingOp, PongOp};

#[deriving(Clone)]
pub enum Payload {
    Text(String),
    Binary(Vec<u8>),
    Empty
}

#[deriving(Clone, FromPrimitive)]
pub enum Opcode {
    ContinuationOp = 0x0,
    TextOp         = 0x1,
    BinaryOp       = 0x2,
    CloseOp        = 0x8,
    PingOp         = 0x9,
    PongOp         = 0xA,
}

// this struct will eventually encapsulate data framing, opcodes, ws extensions, masks etc
// right now, only single frames, with a text payload are supported
#[deriving(Clone)]
pub struct Message {
    pub payload: Payload,
    pub opcode: Opcode
}

impl Message {
    pub fn load(stream: &mut TcpStream) -> IoResult<Box<Message>> {
        let vec1 = try!(stream.read_exact(2));
        let buf1 = vec1.as_slice();

        //let fin = buf1[0] >> 7; // TODO check this, required for handling fragmented messages

        let rsv = (buf1[0] >> 4) & 0b0111;
        if rsv != 0 {
            return Err(IoError {
                kind: InvalidInput,
                desc: "Only RSV=0 allowed, no extension has been negotiated.",
                detail: None
            });
        }

        let opcode = buf1[0] & 0b0000_1111;
        let opcode: Opcode = num::from_u8(opcode).unwrap(); // all valid RFC 6455 Opcodes specified, invalid ones will panic!()

        //let mask    = buf1[1] & 0b1000_0000; TODO use this to determine whether to unmask or not
        let pay_len = buf1[1] & 0b0111_1111;

        let payload_length = match pay_len {
            127 => try!(stream.read_be_u64()), // 8 bytes in network byte order
            126 => try!(stream.read_be_u16()) as u64, // 2 bytes in network byte order
            _   => pay_len as u64
        };

        // payloads larger than 125 bytes are not allowed for control frames
        match opcode {
            CloseOp | PingOp if payload_length > 125 => panic!(),
            _ => ()
        }

        let masking_key = try!(stream.read_exact(4));
        let mut masked_payload_buf = try!(stream.read_exact(payload_length as uint));

        // unmask the payload in-place
        for (i, octet) in masked_payload_buf.iter_mut().enumerate() {
            *octet = *octet ^ masking_key[i % 4];
        }
        let payload_buf = masked_payload_buf;

        let payload: Payload = match opcode {
            TextOp   => Text(String::from_utf8(payload_buf).unwrap()), // unsafe unwrap? failures during autobahn
            BinaryOp => Binary(payload_buf),
            CloseOp  => Empty,
            PingOp   => Binary(payload_buf),
            PongOp   => Binary(payload_buf),
            _        => unimplemented!(), // ContinuationOp
        };

        let message = box Message {
            payload: payload,
            opcode: opcode
        };

        return Ok(message);
    }

    // FIXME support for clients - masking for clients, but need know whether
    // it's a client or server doing the sending. maybe a private `send` with
    // the common code, and public `client_send` and `server_send` methods.
    // these methods will be called by the WebSokcetClient and WebSocketServer
    // traits respectively, and the interface for both clients and servers is
    // the same - just send on the channel, and the trait takes care of it
    pub fn send(&self, stream: &mut TcpStream) -> IoResult<()> {
        let payload_length = match self.payload {
            Text(ref p) => p.len(),
            Binary(ref p) => p.len(),
            Empty => 0,
        };

        try!(stream.write_u8(0b1000_0000 | self.opcode as u8)); // fin: 1, rsv: 000, opcode: see Opcode

        // FIXME: this assumes a server. the first bit, which is the "mask" bit, is implicitly set as 0 here, as required for ws servers
        if payload_length <= 125 {
            try!(stream.write_u8(payload_length as u8));
        } else if payload_length <= 65535 {
            try!(stream.write_u8(126));
            try!(stream.write_be_u16(payload_length as u16));
        } else if payload_length > 65535 {
            try!(stream.write_u8(127));
            try!(stream.write_be_u64(payload_length as u64));
        }

        match self.payload {
            Text(ref p)   => try!(stream.write((*p).as_slice().as_bytes())),
            Binary(ref p) => try!(stream.write((*p).as_slice())),
            Empty => {},
        }

        try!(stream.flush());

        return Ok(());
    }
}
