.PHONY: all

all:
	cargo run

discover:
	cargo run -- discover

refresh:
	cargo run -- refresh

tail:
	cargo run -- tail

format:
	cargo fmt
