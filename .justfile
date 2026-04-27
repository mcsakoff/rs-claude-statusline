##
## General
##

# List available recipes
usage:
    @just --list --unsorted --list-prefix "  " --justfile "{{justfile()}}"

##
## Development
##

# Build the package in release mode, with optimizations
[group("Development")]
build:
    @cargo build --release

# Run the statusline with a test file
[group("Development")]
run TEST_NUM="1":
    @LOG_LEVEL=debug cargo run --bin statusline < ./tests/claude{{TEST_NUM}}.json

# Checks the code to catch common mistakes and improvements
[group("Development")]
check:
    @clear
    @cargo clippy --all -- -W clippy::pedantic -D warnings

# Execute all unit and integration tests
[group("Development")]
test:
    @cargo test
