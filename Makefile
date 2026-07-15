all:
	cargo build --release
	cp target/release/flir-one-pro-lt-v4l2 flirone

clean:
	cargo clean
	rm -f flirone
