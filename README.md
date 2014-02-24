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
