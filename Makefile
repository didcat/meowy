.DEFAULT_GOAL := help

PYTHON ?= python3
CARGO ?= cargo
PREFIX ?= $(HOME)/.local
DESTDIR ?=
LOG_DIR ?= build/verification
VERIFY_ARGS ?=

TARGET := x86_64-unknown-linux-gnu
BINARY := compiler/target/$(TARGET)/release/meowy

.PHONY: help test test-strict build install

help:
	@printf '%s\n' \
	  'make test         Run all checks, including conformance; print results and save logs' \
	  'make test-strict  Run all checks and fail on every unsupported conformance case' \
	  'make build        Build the locked release compiler' \
	  'make install      Build and install to $(PREFIX)/bin/meowy' \
	  'Overrides: PREFIX, DESTDIR, LOG_DIR, VERIFY_ARGS, PYTHON, CARGO'

test:
	@$(PYTHON) -B tools/verify.py --all --keep-going --log-dir "$(LOG_DIR)" $(VERIFY_ARGS)

test-strict:
	@$(PYTHON) -B tools/verify.py --all --strict --keep-going --log-dir "$(LOG_DIR)/strict" $(VERIFY_ARGS)

build:
	$(CARGO) build --release --locked --manifest-path compiler/Cargo.toml --target $(TARGET) --target-dir compiler/target
	@printf 'Built %s\n' "$(abspath $(BINARY))"

install: build
	install -Dm755 "$(BINARY)" "$(DESTDIR)$(PREFIX)/bin/meowy"
	@printf 'Installed %s\n' "$(DESTDIR)$(PREFIX)/bin/meowy"
