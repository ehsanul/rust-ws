//! A Web Socket server

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

impl Server for WebSocketServer {
    fn get_config(&self) -> Config {
        Config { bind_address: SocketAddr { ip: Ipv4Addr(127, 0, 0, 1), port: 8001 } }
    }

    fn handle_request(&self, r: &Request, w: &mut ResponseWriter) {
        match (&r.method, &r.headers.upgrade){
          // (&Get, &Some(~"websocket"), &Some(~[Token(~"Upgrade")])) => { // FIXME this doesn't work. but client must have the header "Connection: Upgrade"
          (&Get, &Some(~"websocket")) => { // TODO client must have the header "Connection: Upgrade"
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
                  //  As an example, if the value of the |Sec-WebSocket-Key|
                  //  header field in the client's handshake were
                  //  "dGhlIHNhbXBsZSBub25jZQ==", the server would append the
                  //  string "258EAFA5-E914-47DA-95CA-C5AB0DC85B11" to form the
                  //  string "dGhlIHNhbXBsZSBub25jZQ==258EAFA5-E914-47DA-95CA-
                  //  C5AB0DC85B11".  The server would then take the SHA-1 hash
                  //  of this string, giving the value 0xb3 0x7a 0x4f 0x2c 0xc0
                  //  0x62 0x4f 0x16 0x90 0xf6 0x46 0x06 0xcf 0x38 0x59 0x45
                  //  0xb2 0xbe 0xc4 0xea.  This value is then base64-encoded,
                  //  to give the value "s3pPLMBiTxaQ9kYGzzhZRbK+xOo=", which
                  //  would be returned in the |Sec-WebSocket-Accept| header
                  //  field.

                  let mut sh = Sha1::new();
                  let mut out = [0u8, ..20];
                  sh.input_str(val + WEBSOCKET_SALT);
                  sh.result(out);
                  let sec_websocket_accept = out.to_base64(STANDARD);
                  debug!("sec websocket accept: {}", sec_websocket_accept);
                  w.headers.insert(ExtensionHeader(~"Sec-WebSocket-Accept", sec_websocket_accept));
                }
                (name, val) => {
                  debug!("{}: {}", name, val);
                }
              }
            }

            return;
          },
          (&_, &Some(_)) => {}, // handle other upgrade - this is rare apparently, but may be used for TLS for example. not sure if browsers actually implement it though.
          (&_, &None) => {} // TODO regular http server should handle this request
        }

        w.headers.date = Some(time::now_utc());
        w.headers.content_type = Some(MediaType {
            type_: ~"text",
            subtype: ~"html",
            parameters: ~[(~"charset", ~"UTF-8")]
        });
        w.headers.server = Some(~"Rust Thingummy/0.0-pre");
        w.write(bytes!("<!DOCTYPE html><title>Rust HTTP server</title>"));

        w.write(bytes!("<h1>Request</h1>"));
        let s = format!("<dl>
            <dt>Method</dt><dd>{}</dd>
            <dt>Host</dt><dd>{:?}</dd>
            <dt>Upgrade</dt><dd>{:?}</dd>
            <dt>Request URI</dt><dd>{:?}</dd>
            <dt>HTTP version</dt><dd>{:?}</dd>
            <dt>Close connection</dt><dd>{}</dd></dl>",
            r.method,
            r.headers.host,
            r.headers.upgrade,
            r.request_uri,
            r.version,
            r.close_connection);
        w.write(s.as_bytes());
        w.write(bytes!("<h2>Extension headers</h2>"));
        w.write(bytes!("<table><thead><tr><th>Name</th><th>Value</th></thead><tbody>"));
        for header in r.headers.iter() {
            let line = format!("<tr><td><code>{}</code></td><td><code>{}</code></td></tr>",
                               header.header_name(),
                               header.header_value());
            w.write(line.as_bytes());
        }
        w.write(bytes!("</tbody></table>"));
        w.write(bytes!("<h2>Body</h2><pre>"));
        w.write(r.body.as_bytes());
        w.write(bytes!("</pre>"));

        w.write(bytes!("<h1>Response</h1>"));
        let s = format!("<dl><dt>Status</dt><dd>{}</dd></dl>", w.status);
        w.write(s.as_bytes());
        w.write(bytes!("<h2>Headers</h2>"));
        w.write(bytes!("<table><thead><tr><th>Name</th><th>Value</th></thead><tbody>"));
        {
            let h = w.headers.clone();
            for header in h.iter() {
                let line = format!("<tr><td><code>{}</code></td><td><code>{}</code></td></tr>",
                                header.header_name(),
                                header.header_value());
                w.write(line.as_bytes());
            }
        }
        w.write(bytes!("</tbody></table>"));
    }
}

fn main() {
    WebSocketServer.serve_forever();
}
