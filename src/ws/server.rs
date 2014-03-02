use rust_crypto::sha1::Sha1;
use rust_crypto::digest::Digest;
use serialize::base64::{ToBase64, STANDARD};
use time;

use std::io::{Listener, Acceptor};
use std::io::net::tcp::TcpListener;

use http::buffer::BufferedStream;
use std::io::net::tcp::TcpStream;

use http::server::{Server, Request, ResponseWriter};
use http::status::SwitchingProtocols;
use http::headers::HeaderEnum;
use http::headers::response::ExtensionHeader;
use http::headers::connection::Token;
use http::method::Get;

use message::Message;

static WEBSOCKET_SALT: &'static str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

pub trait WebSocketServer: Server {
    // called when a web socket connection is successfully established.
    //
    // this can't block! leaving implementation to trait user, in case they
    // want to custom scheduling, tracking clients, reconnect logic, etc.
    //
    // TODO: may want to send more info in, such as the connecting IP address?
    fn handle_ws_connect(&self, receiver: Port<~Message>, sender: Chan<~Message>) -> ();

    // XXX: this is mostly a copy of the serve_forever fn in the Server trait.
    //      rust-http needs some changes in order to avoid this duplication
    fn ws_serve_forever(self) {
        let config = self.get_config();
        debug!("About to bind to {:?}", config.bind_address);
        let mut acceptor = match TcpListener::bind(config.bind_address).listen() {
            Err(err) => {
                error!("bind or listen failed :-(: {}", err);
                return;
            },
            Ok(acceptor) => acceptor,
        };
        debug!("listening");
        loop {
            let stream = match acceptor.accept() {
                Err(error) => {
                    debug!("accept failed: {:?}", error);
                    // Question: is this the correct thing to do? We should probably be more
                    // intelligent, for there are some accept failures that are likely to be
                    // permanent, such that continuing would be a very bad idea, such as
                    // ENOBUFS/ENOMEM; and some where it should just be ignored, e.g.
                    // ECONNABORTED. TODO.
                    continue;
                },
                Ok(socket) => socket,
            };
            let child_self = self.clone();
            spawn(proc() {
                let mut stream = BufferedStream::new(stream);
                debug!("accepted connection, got {:?}", stream);

                let mut successful_handshake = false;
                loop {  // A keep-alive loop, condition at end
                    let (request, err_status) = Request::load(&mut stream);
                    let mut response = ~ResponseWriter::new(&mut stream, request);
                    match err_status {
                        Ok(()) => {
                            successful_handshake = child_self.handle_possible_ws_request(request, response);
                            // Ensure that we actually do send a response:
                            match response.try_write_headers() {
                                Err(err) => {
                                    error!("Writing headers failed: {}", err);
                                    return;  // Presumably bad connection, so give up.
                                },
                                Ok(_) => (),
                            }
                        },
                        Err(status) => {
                            // Uh oh, it's a response that I as a server cannot cope with.
                            // No good user-agent should have caused this, so for the moment
                            // at least I am content to send no body in the response.
                            response.status = status;
                            response.headers.content_length = Some(0);
                            match response.write_headers() {
                                Err(err) => {
                                    error!("Writing headers failed: {}", err);
                                    return;  // Presumably bad connection, so give up.
                                },
                                Ok(_) => (),
                            }
                        },
                    }
                    // Ensure the request is flushed, any Transfer-Encoding completed, etc.
                    match response.finish_response() {
                        Err(err) => {
                            error!("finishing response failed: {}", err);
                            return;  // Presumably bad connection, so give up.
                        },
                        Ok(_) => (),
                    }

                    if successful_handshake || request.close_connection {
                        break;
                    }
                }

                if successful_handshake {
                    child_self.serve_websockets(stream);
                }
            });
        }
    }

    fn serve_websockets(&self, stream: BufferedStream<TcpStream>) {
        let mut stream = stream.wrapped;
        let write_stream = stream.clone();
        let (in_receiver, in_sender) = Chan::new();
        let (out_receiver, out_sender) = Chan::new();

        self.handle_ws_connect(in_receiver, out_sender);

        // write task
        spawn(proc() {
            // ugh: https://github.com/mozilla/rust/blob/3dbc1c34e694f38daeef741cfffc558606443c15/src/test/run-pass/kindck-implicit-close-over-mut-var.rs#L40-L44
            // work to fix this is ongoing here: https://github.com/mozilla/rust/issues/11958
            let mut write_stream = write_stream;

            loop {
                let message = out_receiver.recv();
                message.send(&mut write_stream).unwrap(); // fails this task in case of an error; FIXME make sure this fails the read (parent) task
            }
        });

        // read task, effectively the parent of the write task
        loop {
            let message = Message::load(&mut stream).unwrap(); // fails the task if there's an error. FIXME make sure this fails the write task
            debug!("message: {:?}", message);
            in_sender.send(message);
        }
    }

    fn sec_websocket_accept(&self, sec_websocket_key: &str) -> ~str {
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

    // check if the http request is a web socket upgrade request, and return true if so.
    // otherwise, fall back on the regular http request handler
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
                w.headers.server = Some(~"rust-ws/0.1-pre");

                for header in r.headers.iter() {
                    debug!("{}: {}", header.header_name(), header.header_value());
                }

                // NOTE: think this is actually Sec-WebSocket-Key (capital Web[S]ocket), but rust-http normalizes header names
                match r.headers.extensions.find(&~"Sec-Websocket-Key") {
                    Some(val) => {
                        let sec_websocket_accept = self.sec_websocket_accept(*val);
                        w.headers.insert(ExtensionHeader(~"Sec-WebSocket-Accept", sec_websocket_accept));
                    },
                    None => fail!()
                }

                return true; // successful_handshake
            },
            (&_, &_) => self.handle_request(r, w)
        }
        return false;
    }
}
