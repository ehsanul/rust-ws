//! A WebSocket Server

#[crate_id = "echo"];

extern mod extra;
extern mod http;
extern mod ws;

use http::server::{Config, Server, Request, ResponseWriter};
use std::io::net::ip::{SocketAddr, Ipv4Addr};
use http::headers::content_type::MediaType;
use extra::time;

use ws::server::WebSocketServer;
use ws::message::{Message, TextOp, Text, BinaryOp, Binary};

#[deriving(Clone)]
struct EchoServer;

impl Server for EchoServer {
    fn get_config(&self) -> Config {
        Config { bind_address: SocketAddr { ip: Ipv4Addr(127, 0, 0, 1), port: 8001 } }
    }

    fn handle_request(&self, r: &Request, w: &mut ResponseWriter) {
        w.headers.date = Some(time::now_utc());
        w.headers.content_type = Some(MediaType {
            type_: ~"text",
            subtype: ~"html",
            parameters: ~[(~"charset", ~"UTF-8")]
        });
        w.headers.server = Some(~"EchoServer");

        w.write(bytes!("<h1>Echo Server</h1>")).unwrap();
        w.write(bytes!("<script>ws = new WebSocket('ws://localhost:8001/'); ws.onmessage = function(x){console.log(x)}; setInterval(function(){ ws.send('hi! ' + Math.random().toString()); }, 1000)</script>")).unwrap();
    }
}

impl WebSocketServer for EchoServer {
    fn handle_ws_connect(&self, receiver: Port<~Message>, sender: Chan<~Message>) {
        spawn(proc() {
            loop {
                let message = receiver.recv();
                let (payload, opcode) = match message.payload {
                    Text(p)   => (Text("Echo: " + p), TextOp),
                    Binary(p) => (Binary(p), BinaryOp),
                    //_         => unimplemented!(), // this is unreachable for now due to server refusing to pass other opcodes
                };
                let echo_message = ~Message {
                    payload: payload,
                    opcode: opcode,
                };
                sender.send(echo_message);
            }
        });
    }
}

fn main() {
    EchoServer.ws_serve_forever();
}
