//! A WebSocket Server

#[crate_id = "rust-ws"];

extern mod extra;
extern mod http;
extern mod rust_crypto = "rust-crypto";

use std::str::from_utf8;

use rust_crypto::sha1::Sha1;
use rust_crypto::digest::Digest;
use extra::base64::{ToBase64, STANDARD};

use std::io::net::ip::{SocketAddr, Ipv4Addr};
use std::io::Writer;
use extra::time;

use std::comm::SharedChan;
use std::io::{Listener, Acceptor};
use std::io::io_error;
use std::io::net::tcp::TcpListener;

use http::buffer::BufferedStream;
use std::io::net::tcp::TcpStream;

use http::server::{Config, Server, Request, ResponseWriter};
use http::status::SwitchingProtocols;
use http::headers::HeaderEnum;
use http::headers::response::ExtensionHeader;
use http::headers::content_type::MediaType;
use http::headers::connection::Token;
use http::method::Get;

static WEBSOCKET_SALT: &'static str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

enum WebSocketState {
  WSConnecting, WSOpen, WSClosing, WSClosed
}

trait WebSocketServer: Server {

    // this is mostly a copy of the serve_forever fn in the Server trait
    fn override_serve_forever(self) {
        let config = self.get_config();
        debug!("About to bind to {:?}", config.bind_address);
        let mut acceptor = match TcpListener::bind(config.bind_address).listen() {
            None => {
                error!("bind or listen failed :-(");
                return;
            },
            Some(acceptor) => acceptor,
        };
        debug!("listening");
        loop {
            // OK, we're sort of shadowing an IoError here. Perhaps this should be done in a
            // separate task so that it can safely fail...
            let mut error = None;
            let optstream = io_error::cond.trap(|e| {
                error = Some(e);
            }).inside(|| {
                acceptor.accept()
            });

            if optstream.is_none() {
                debug!("accept failed: {:?}", error);
                // Question: is this the correct thing to do? We should probably be more
                // intelligent, for there are some accept failures that are likely to be
                // permanent, such that continuing would be a very bad idea, such as
                // ENOBUFS/ENOMEM; and some where it should just be ignored, e.g.
                // ECONNABORTED. TODO.
                continue;
            }
            let child_self = self.clone();
            do spawn {
                let mut stream = BufferedStream::new(optstream.unwrap());
                debug!("accepted connection, got {:?}", stream);

                let mut successful_handshake = false;
                loop {  // A keep-alive loop, condition at end
                    let (request, err_status) = Request::load(&mut stream);
                    let mut response = ~ResponseWriter::new(&mut stream, request);
                    match err_status {
                        Ok(()) => {
                            successful_handshake = child_self.handle_possible_ws_request(request, response);
                            // Ensure that we actually do send a response:
                            response.try_write_headers();
                        },
                        Err(status) => {
                            // Uh oh, it's a response that I as a server cannot cope with.
                            // No good user-agent should have caused this, so for the moment
                            // at least I am content to send no body in the response.
                            response.status = status;
                            response.headers.content_length = Some(0);
                            response.write_headers();
                        },
                    }
                    // Ensure the request is flushed, any Transfer-Encoding completed, etc.
                    response.finish_response();

                    if successful_handshake || request.close_connection {
                        break;
                    }
                }

                if successful_handshake {
                    child_self.serve_websockets(&mut stream);
                }
            }
        }
    }

    fn serve_websockets(&self, stream: &mut BufferedStream<TcpStream>) {
        let mut status = WSOpen;
        loop {
            let buf1 = stream.read_bytes(2); // FIXME trap io_error condition
            debug!("buf1: {:t} {:t}", buf1[0], buf1[1]);

            let fin    = buf1[0] & 0b1000_0000; // TODO check this, required for handling fragmented messages
            let rsv1   = buf1[0] & 0b0100_0000;
            let rsv2   = buf1[0] & 0b0010_0000;
            let rsv3   = buf1[0] & 0b0001_0000;
            let opcode = buf1[0] & 0b0000_1111;

            let mask    = buf1[1] & 0b1000_0000;
            let pay_len = buf1[1] & 0b0111_1111;

            let payload_length = match pay_len {
                127 => stream.read_be_u64(), // 8 bytes in network byte order
                126 => stream.read_be_u16() as u64, // 2 bytes in network byte order
                _   => pay_len as u64
            };
            debug!("payload_length: {}", payload_length);

            let masking_key_buf = stream.read_bytes(4);
            debug!("masking_key_buf: {:t} {:t} {:t} {:t}", masking_key_buf[0], masking_key_buf[1], masking_key_buf[2], masking_key_buf[3]);

            let masked_payload_buf = stream.read_bytes(payload_length as uint); // FIXME payload_length could be upto 64 bits, so this could fail on archs with a 32-bit uint

            // unmask the payload
            let mut payload_buf = ~[]; // ugh, a map_with_index would be nice. or maybe just mutate the existing buffer in place.
            for (i, &octet) in masked_payload_buf.iter().enumerate() {
                payload_buf.push(octet ^ masking_key_buf[i % 4]);
            }

            let payload = from_utf8(payload_buf); // FIXME could be text OR binary! look at opcode to know which
            debug!("payload: {}", payload);
        }
    }

    fn sec_websocket_accept(&self, sec_websocket_key: ~str) -> ~str {
        // NOTE from RFC 6455
        //
        // To prove that the handshake was received, the server has to take two
        // pieces of information and combine them to form a response.  The first
        // piece of information comes from the |Sec-WebSocket-Key| header field
        // in the client handshake:
        //
        //      Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==
        //
        // For this header field, the server has to take the value (as present
        // in the header field, e.g., the base64-encoded [RFC4648] version minus
        // any leading and trailing whitespace) and concatenate this with the
        // Globally Unique Identifier (GUID, [RFC4122]) "258EAFA5-E914-47DA-
        // 95CA-C5AB0DC85B11" in string form, which is unlikely to be used by
        // network endpoints that do not understand the WebSocket Protocol.  A
        // SHA-1 hash (160 bits) [FIPS.180-3], base64-encoded (see Section 4 of
        // [RFC4648]), of this concatenation is then returned in the server's
        // handshake.

        let mut sh = Sha1::new();
        let mut out = [0u8, ..20];
        sh.input_str(sec_websocket_key + WEBSOCKET_SALT);
        sh.result(out);
        return out.to_base64(STANDARD);
    }

    fn handle_possible_ws_request(&self, r: &Request, w: &mut ResponseWriter) -> bool {
        // TODO allow configuration of endpoint for websocket
        match (&r.method, &r.headers.upgrade){
            // (&Get, &Some(~"websocket"), &Some(~[Token(~"Upgrade")])) => //\{ FIXME this doesn't work. but client must have the header "Connection: Upgrade"
            (&Get, &Some(~"websocket")) => {
                // TODO client must have the header "Connection: Upgrade"
                //
                // TODO The request MUST include a header field with the name
                // |Sec-WebSocket-Version|. The value of this header field MUST be 13.

                // WebSocket Opening Handshake
                w.status = SwitchingProtocols;
                w.headers.upgrade = Some(~"websocket");
                // w.headers.transfer_encoding = None;
                w.headers.content_length = Some(0);
                w.headers.connection = Some(~[Token(~"Upgrade")]);
                w.headers.date = Some(time::now_utc());
                w.headers.server = Some(~"rust-ws/0.0-pre");

                // FIXME must we iter?
                for header in r.headers.iter() {
                    match (header.header_name(), header.header_value()) {
                        (~"Sec-Websocket-Key", val) => {
                            let sec_websocket_accept = self.sec_websocket_accept(val);
                            debug!("sec websocket accept: {}", sec_websocket_accept);
                            w.headers.insert(ExtensionHeader(~"Sec-WebSocket-Accept", sec_websocket_accept));
                        }
                        (name, val) => {
                            debug!("{}: {}", name, val);
                        }
                    }
                }
                return true; // successful_handshake
            },
            (&_, &_) => self.handle_request(r, w)
        }
        return false;
    }
}


#[deriving(Clone)]
struct ExampleWSServer;

impl WebSocketServer for ExampleWSServer { }

impl Server for ExampleWSServer {
    fn get_config(&self) -> Config {
        Config { bind_address: SocketAddr { ip: Ipv4Addr(127, 0, 0, 1), port: 8001 } }
    }

    // dummy method is required since the WebSocketServer trait cannot override
    // a default method on the Server trait
    fn serve_forever(self) {
      self.override_serve_forever();
    }

    fn handle_request(&self, r: &Request, w: &mut ResponseWriter) {
        w.headers.date = Some(time::now_utc());
        w.headers.server = Some(~"rust-ws/0.0-pre");
        w.headers.content_type = Some(MediaType {
            type_: ~"text",
            subtype: ~"html",
            parameters: ~[(~"charset", ~"UTF-8")]
        });
        w.write(bytes!("<!DOCTYPE html><title>Rust WebSocket Server</title><h1>Rust WebSocket Server</h1>"));
    }
}

fn main() {
    ExampleWSServer.serve_forever();
}
