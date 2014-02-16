//! A WebSocket Server

#[crate_id = "wsexample"];

extern mod extra;
extern mod http;
extern mod ws;

use http::server::{Config, Server, Request, ResponseWriter};
use std::io::net::ip::{SocketAddr, Ipv4Addr};
use http::headers::content_type::MediaType;
use extra::time;
use ws::server::WebSocketServer;

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
