#!/usr/bin/env bash

set -euo pipefail

cargo fmt
cargo clippy -- -D warnings

export RUST_BACKTRACE=1
RUST_BACKTRACE=1 cargo test

jj desc
jj new
git push origin HEAD:refs/heads/master
git checkout master
git pull