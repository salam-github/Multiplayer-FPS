#!/bin/bash

# Hardcoded path relative to the root
PROJECT_PATH_1="fps-vilburg/server"

# Open a new Terminal window and run the Rust program
open -a Terminal "$PROJECT_PATH_1" --args cargo run --release
