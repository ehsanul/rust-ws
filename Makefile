RUSTC ?= rustc
RUSTFLAGS ?= -O -Z debug-info -L ../rust-http/build/ -L ../rust-crypto/

libws_so=build/libws-8adb277a-0.1-pre.dylib

all: ws examples

ws: $(libws_so)

$(libws_so):
	mkdir -p build/
	$(RUSTC) $(RUSTFLAGS) src/ws/lib.rs --out-dir build

build/%:: src/%/main.rs $(libws_so)
	mkdir -p "$(dir $@)"
	$(RUSTC) $(RUSTFLAGS) $< -o $@ -L build/

examples: $(patsubst src/examples/%/main.rs,build/examples/%,$(wildcard src/examples/*/main.rs)) \
		  $(patsubst src/examples/%/main.rs,build/examples/%,$(wildcard src/examples/*/*/main.rs))

clean:
	rm -rf build/

.PHONY: all ws examples clean
