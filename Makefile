RUSTC ?= rustc
RUSTFLAGS ?= -O -L lib/rust-http/build/ -L ../rust-crypto/

libws_so=build/libws-8adb277a-0.1-pre.dylib

all: deps ws examples

deps: rust-http

rust-http:
	make -C lib/rust-http http

ws: $(libws_so)

$(libws_so): $(wildcard src/ws/*.rs) $(wildcard src/ws/server/*.rs)
	mkdir -p build/
	$(RUSTC) $(RUSTFLAGS) src/ws/lib.rs --out-dir build

build/%:: src/%/main.rs $(libws_so)
	mkdir -p "$(dir $@)"
	$(RUSTC) $(RUSTFLAGS) $< -o $@ -L build/

examples: $(patsubst src/examples/%/main.rs,build/examples/%,$(wildcard src/examples/*/main.rs)) \
		  $(patsubst src/examples/%/main.rs,build/examples/%,$(wildcard src/examples/*/*/main.rs))

clean:
	rm -rf build/
	make -C lib/rust-http clean

.PHONY: all ws examples clean
