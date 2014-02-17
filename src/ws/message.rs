use std::io::net::tcp::TcpStream;
use std::io::IoResult;
use std::str;

// this struct will eventually encapsulate data framing, opcodes, ws extensions, masks etc
// right now, only single frames, with a text payload are supported
pub struct Message {
    payload: ~str // TODO: make this a Payload enum or something
}

impl Message {
    pub fn load(stream: &mut TcpStream) -> IoResult<~Message> {
        let buf1 = if_ok!(stream.read_bytes(2));
        debug!("buf1: {:t} {:t}", buf1[0], buf1[1]);

        let fin    = buf1[0] & 0b1000_0000; // TODO check this, required for handling fragmented messages
        /* we ignore these, as they are only used if a websocket protocol has been enabled, and optionally at that
        let rsv1   = buf1[0] & 0b0100_0000;
        let rsv2   = buf1[0] & 0b0010_0000;
        let rsv3   = buf1[0] & 0b0001_0000;
        */
        let opcode = buf1[0] & 0b0000_1111; // TODO check for ping/pong/text/binary

        let mask    = buf1[1] & 0b1000_0000;
        let pay_len = buf1[1] & 0b0111_1111;

        let payload_length = match pay_len {
            127 => if_ok!(stream.read_be_u64()), // 8 bytes in network byte order
            126 => if_ok!(stream.read_be_u16()) as u64, // 2 bytes in network byte order
            _   => pay_len as u64
        };
        debug!("payload_length: {}", payload_length);

        let masking_key_buf = if_ok!(stream.read_bytes(4));
        debug!("masking_key_buf: {:t} {:t} {:t} {:t}", masking_key_buf[0], masking_key_buf[1], masking_key_buf[2], masking_key_buf[3]);

        let masked_payload_buf = if_ok!(stream.read_bytes(payload_length as uint)); // FIXME payload_length could be upto 64 bits, so this could fail on archs with a 32-bit uint

        // unmask the payload
        let mut payload_buf = ~[]; // instead of a mutable vector, a map_with_index would be nice. or maybe just mutate the existing buffer in place.
        for (i, &octet) in masked_payload_buf.iter().enumerate() {
            payload_buf.push(octet ^ masking_key_buf[i % 4]);
        }

        let payload = str::from_utf8_owned(payload_buf).unwrap(); // FIXME shouldn't just unwrap? also, could be text OR binary! look at opcode to know which

        let message = ~Message {
            payload: payload
        };

        return Ok(message);
    }

    // FIXME support for clients - masking for clients, but need know whether
    // it's a client or server doing the sending. maybe a private `send` with
    // the common code, and public `client_send` and `server_send` methods
    pub fn send(&self, stream: &mut TcpStream) -> IoResult<()> {
        let payload_length = self.payload.len(); // XXX len() returns a uint, so i'm guessing this doesn't work for extremely large payloads. in ws, payload length itself may be upto 64 bits. ie a 2gb+ message fails

        if_ok!(stream.write_u8(0b1000_0001)); // fin: 1, rsv: 000, opcode: 0001 (text frame) - TODO choose binary vs text based on payload's type, and allow other opcodes too

        // FIXME: this assumes a server. the first bit, which is the "mask" bit, is implicitly set as 0 here, as required for ws servers
        if payload_length <= 125 {
            if_ok!(stream.write_u8(payload_length as u8));
        } else if payload_length <= 65536 {
            if_ok!(stream.write_u8(126));
            if_ok!(stream.write_be_u16(payload_length as u16));
        } else if payload_length <= 65536 {
            if_ok!(stream.write_u8(127));
            if_ok!(stream.write_be_u64(payload_length as u64));
        }

        if_ok!(stream.write_str(self.payload)); // TODO support binary payload
        if_ok!(stream.flush());

        return Ok(());
    }
}
