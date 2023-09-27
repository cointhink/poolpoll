.PHONY: all

all:
	cargo run

discover:
	cargo run -- discover

refresh:
	cargo run -- refresh

format:
	cargo fmt
