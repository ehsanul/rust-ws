RUSTC ?= rustc
RUSTFLAGS ?= -O -L lib/rust-http/build/ -L lib/rust-crypto/

libws_so=build/libws-8adb277a-0.1-pre.dylib

all: deps ws examples

deps: rust-http rust-crypto

rust-http:
	make -C lib/rust-http http

rust-crypto:
	make -C lib/rust-crypto rust-crypto

ws: $(libws_so)

$(libws_so): $(wildcard src/ws/*.rs) $(wildcard src/ws/server/*.rs)
	mkdir -p build/
	$(RUSTC) $(RUSTFLAGS) src/ws/lib.rs --out-dir build

build/%:: src/%/main.rs $(libws_so)
	mkdir -p "$(dir $@)"
	$(RUSTC) $(RUSTFLAGS) $< -o $@ -L build/

examples: $(patsubst src/examples/%/main.rs,build/examples/%,$(wildcard src/examples/*/main.rs)) \
		  $(patsubst src/examples/%/main.rs,build/examples/%,$(wildcard src/examples/*/*/main.rs))

clean: clean-ws clean-deps

clean-ws:
	rm -rf build/

clean-deps: clean-rust-http clean-rust-crypto

clean-rust-http:
	make -C lib/rust-http clean

clean-rust-crypto:
	make -C lib/rust-crypto clean

.PHONY: all ws examples clean
