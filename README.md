# rust-ws

rust-ws is a really basic, and incomplete web sockets library for Rust. Text
and binary messages are supported, and only servers are supported for now.
Clients (and other support) coming soon. See the Todo section below for what's
missing, there's a lot to do!

## Build

You need to have a bleeding-edge rustc already in your `PATH` first, though
this repository may be somewhat behind master at times. Cargo will fetch other
dependancies.

    git clone git@github.com:ehsanul/rust-ws.git
    cd rust-ws
    cargo build
    cargo test # builds examples in test/

## Examples

See the echo server example under the `examples/` directory for basic usage.

## Todo

So much to do.

### Code

- Cleanly drop associated tasks - ie when the read task fails, the write task
  ought to be `fail!`-ed, even if it is waiting on a separate Chan read, and
  vice-versa. Right now, this is not handled properly, leading to a memory leak
  for every new ws connection.
- ws clients. Right now, it assumes a ws server, but a lot of the code is
  common to both clients and servers. factor that out and create
  a `WebSocketClient` trait.
- Get closer to RFC 6455 in terms of what connections are accepted and what are
  rejected. right now, some invalid things are allowed. eg there's no check for
  `Connection: Upgrade` header, or the `Sec-WebSocket-Version: 13` header.
- Unit tests.
- Benchmarks, comparisons, optimization.

### Missing Support

- Closing handshake
- Ping/pong frames
- Fragmented messages
- Pass http://autobahn.ws/testsuite/
- TLS
- ws extensions, like deflate etc that chrome/ff use
  - Chrome has
    `Sec-Websocket-Extensions: permessage-deflate; client_max_window_bits, x-webkit-deflate-frame`
- `Sec-WebSocket-Protocol`? don't know if anyone uses that, despite it's presence in the RFC

### More Examples

- Chat example
- Cursor sharing example
- Demo of 3d physics server for mmo game
