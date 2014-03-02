# rust-ws

rust-ws is a really basic, and incomplete web sockets library for Rust. Text
and binary messages are supported, and only servers are supported for now.
Clients (and other support) coming soon. See the TODO file for what's missing,
there's a lot to do!

## Build

You need to have a bleeding-edge rustc already in your `PATH` first, though
this repository may be a week or two behind master at times. Other dependancies
(rust-http and rust-crypto) are included as submodules already.

    git clone --recursive git@github.com:ehsanul/rust-ws.git
    cd rust-ws
    make all

## Examples

See the echo server example under `src/examples` for basic usage.

## Todo

So much to do.

### Code

- changes to rust-http to enable protocol upgrades without the current hacky
  approach in rust-ws
- cleanly drop associated tasks - ie when the read task fails, the write task
  ought to be fail!-ed, even if it is waiting on a separate Chan read, and
  vice-versa. right now, this is not handled properly, leading to a memory leak
  for every new ws connection
- ws clients. right now, it assumes a ws server, but a lot of the code is
  common to both clients and servers. factor that out and create
  a WebSocketClient trait
- get closer to RFC 6455 in terms of what connections are accepted and what are
  rejected. right now, some invalid things are allowed. eg there's no check for
  `Connection: Upgrade` header, or the `Sec-WebSocket-Version: 13` header
- internal unit tests
- benchmarks, comparisons, optimization

### Missing Support

- closing handshake
- ping/pong frames
- fragmented messages
- pass http://autobahn.ws/testsuite/
- TLS
- ws extensions, like deflate etc that chrome/ff use
  - Chrome has
    `Sec-Websocket-Extensions: permessage-deflate; client_max_window_bits, x-webkit-deflate-frame`
- Sec-WebSocket-Protocol? don't know if anyone uses that, despite it's presence in the RFC

### More Examples

- chat example
- cursor sharing example
- demo of 3d physics server for mmo game
