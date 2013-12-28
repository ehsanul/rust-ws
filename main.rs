//! A WebSocket Server

#[crate_id = "rust-ws"];

extern mod extra;
extern mod http;
extern mod rust_crypto = "rust-crypto";

use rust_crypto::sha1::Sha1;
use rust_crypto::digest::Digest;
use extra::base64::{ToBase64, STANDARD};

use std::io::net::ip::{SocketAddr, Ipv4Addr};
use std::io::Writer;
use extra::time;

use http::server::{Config, Server, Request, ResponseWriter};
use http::status::SwitchingProtocols;
use http::headers::HeaderEnum;
use http::headers::response::ExtensionHeader;
use http::headers::content_type::MediaType;
use http::headers::connection::Token;
use http::method::Get;

static WEBSOCKET_SALT: &'static str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

#[deriving(Clone)]
struct WebSocketServer;

trait HandleHTTP {
    fn handle_http_request(&self, r: &Request, w: &mut ResponseWriter);
}

impl HandleHTTP for WebSocketServer {
    fn handle_http_request(&self, r: &Request, w: &mut ResponseWriter) {
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

impl Server for WebSocketServer {
    fn get_config(&self) -> Config {
        Config { bind_address: SocketAddr { ip: Ipv4Addr(127, 0, 0, 1), port: 8001 } }
    }

    fn handle_request(&self, r: &Request, w: &mut ResponseWriter) {
        // TODO allow configuration of endpoint for websocket
        match (&r.method, &r.headers.upgrade){
            // (&Get, &Some(~"websocket"), &Some(~[Token(~"Upgrade")])) => { // FIXME this doesn't work. but client must have the header "Connection: Upgrade"
            (&Get, &Some(~"websocket")) => { // TODO client must have the header "Connection: Upgrade"
                // WebSocket Opening Handshake
                w.status = SwitchingProtocols;
                w.headers.upgrade = Some(~"websocket");

                // w.headers.transfer_encoding = None;
                w.headers.content_length = Some(0);

                w.headers.connection = Some(~[Token(~"Upgrade")]);

                // FIXME must we iter?
                for header in r.headers.iter() {
                    match (header.header_name(), header.header_value()) {
                        (~"Sec-Websocket-Key", val) => {
                            //  NOTE from RFC 6455
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
                            sh.input_str(val + WEBSOCKET_SALT);
                            sh.result(out);
                            let sec_websocket_accept = out.to_base64(STANDARD);
                            debug!("sec websocket accept: {}", sec_websocket_accept);
                            w.headers.insert(ExtensionHeader(~"Sec-WebSocket-Accept", sec_websocket_accept));

                            w.headers.date = Some(time::now_utc());
                            w.headers.server = Some(~"rust-ws/0.0-pre");
                        }
                        (name, val) => {
                            debug!("{}: {}", name, val);
                        }
                    }
                }
            },
            (&_, &_) => self.handle_http_request(r, w)
        }
    }
}

fn main() {
    WebSocketServer.serve_forever();
}
