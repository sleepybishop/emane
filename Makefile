.PHONY: all build clean check

all: build

build:
	cd rust && cargo build --workspace

check:
	cd rust && cargo check --workspace

clean:
	cd rust && cargo clean
