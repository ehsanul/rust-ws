//! A WebSocket Echo Server

extern crate time;
extern crate http;
extern crate ws;

use std::thread::Thread;
use std::sync::mpsc::{channel, Sender, Receiver};

use http::server::{Config, Server, Request, ResponseWriter};
use std::io::net::ip::{SocketAddr, Ipv4Addr};
use http::headers::content_type::MediaType;

use ws::server::WebSocketServer;
use ws::message::{Message, TextOp, Text, BinaryOp, Binary};

#[deriving(Clone)]
struct EchoServer;

impl Server for EchoServer {
    fn get_config(&self) -> Config {
        Config { bind_address: SocketAddr { ip: Ipv4Addr(127, 0, 0, 1), port: 8001 } }
    }

    fn handle_request(&self, r: Request, w: &mut ResponseWriter) {
        w.headers.date = Some(time::now_utc());
        w.headers.content_type = Some(MediaType {
            type_: String::from_str("text"),
            subtype: String::from_str("html"),
            parameters: vec!((String::from_str("charset"), String::from_str("UTF-8"))),
        });
        w.headers.server = Some(String::from_str("EchoServer"));

        w.write(b"<h1>Echo Server</h1>").unwrap();
        w.write(b"<script>count = 0; interval = setInterval(function(){ ws = new WebSocket('ws://localhost:8001/'); count++; if (count > 300){ clearInterval(interval) } }, 10)</script>").unwrap();
    }
}

impl WebSocketServer for EchoServer {
    fn handle_ws_connect(&self, receiver: Receiver<Box<Message>>, sender: Sender<Box<Message>>) {
        Thread::spawn(move || {
            loop {
                let message = receiver.recv().unwrap();

                let (payload, opcode) = match message.payload {
                    Text(p)   => (Text(p), TextOp),
                    Binary(p) => (Binary(p), BinaryOp),
                    _         => unimplemented!(),
                };
                let echo_message = box Message {
                    payload: payload,
                    opcode: opcode,
                };
                sender.send(echo_message).unwrap();
            }
        }).detach();
    }
}

fn main() {
    EchoServer.ws_serve_forever();
}
