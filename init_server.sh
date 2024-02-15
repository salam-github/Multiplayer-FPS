#!/bin/bash

# Hardcoded path relative to the root
PROJECT_PATH_1="$1"
PORT="$2"

# Create and execute an AppleScript to open a new Terminal window and run the Rust program
osascript -e "tell application \"Terminal\" to do script \"cd $PROJECT_PATH_1 && cargo run --release $PORT\""
