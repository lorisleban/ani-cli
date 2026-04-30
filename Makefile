# ani-cli Makefile

BINARY_NAME = ani-cli
RELEASE_DIR = ./target/release
INSTALL_DIR = /usr/local/bin

.PHONY: all build install uninstall clean fmt check

all: build

# Build release binary
build:
	@echo "Building release binary..."
	cargo build --release

# Build and install globally
install: build
	@echo "Installing $(BINARY_NAME) to $(INSTALL_DIR)..."
	sudo cp $(RELEASE_DIR)/$(BINARY_NAME) $(INSTALL_DIR)/
	@echo "Installed! Run '$(BINARY_NAME)' to start."

# Install to ~/.local/bin (no sudo required)
install-local: build
	@echo "Installing $(BINARY_NAME) to ~/.local/bin..."
	mkdir -p ~/.local/bin
	cp $(RELEASE_DIR)/$(BINARY_NAME) ~/.local/bin/
	@echo "Installed to ~/.local/bin! Make sure it's in your PATH."

# Uninstall from /usr/local/bin
uninstall:
	@echo "Removing $(BINARY_NAME) from $(INSTALL_DIR)..."
	sudo rm -f $(INSTALL_DIR)/$(BINARY_NAME)
	@echo "Uninstalled."

# Uninstall from ~/.local/bin
uninstall-local:
	@echo "Removing $(BINARY_NAME) from ~/.local/bin..."
	rm -f ~/.local/bin/$(BINARY_NAME)
	@echo "Uninstalled."

# Clean build artifacts
clean:
	cargo clean

# Format code
fmt:
	cargo fmt

# Run clippy lints
check:
	cargo clippy -- -D warnings

# Build, install, and run
run: install
	$(BINARY_NAME)
