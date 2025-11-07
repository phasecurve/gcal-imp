.PHONY: help build test run clean check fmt lint prod-build

help:
	@echo "gcal-imp - Google Calendar TUI"
	@echo ""
	@echo "Available targets:"
	@echo "  build       - Build the project (debug)"
	@echo "  prod-build  - Build release binary"
	@echo "  test        - Run all tests"
	@echo "  run         - Run the application"
	@echo "  clean       - Clean build artifacts"
	@echo "  check       - Run cargo check"
	@echo "  fmt         - Format code"
	@echo "  lint        - Run clippy linter"
	@echo ""
	@echo "Installation:"
	@echo "  Linux/macOS: ./install.sh"
	@echo "  Windows:     install.bat"

build:
	cargo build

prod-build:
	cargo build --release

test:
	cargo test

run:
	cargo run

clean:
	cargo clean

check:
	cargo check

fmt:
	cargo fmt

lint:
	cargo clippy -- -D warnings


