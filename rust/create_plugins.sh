#!/bin/bash
set -e

# bentpipe
mkdir -p plugins/bentpipe/src
cat << 'TOML' > plugins/bentpipe/Cargo.toml
[package]
name = "bentpipe"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
emane-core = { path = "../../emane-core" }
libc = "0.2"
TOML
mv emane-core/src/bentpipe/* plugins/bentpipe/src/ 2>/dev/null || true
sed -i '/pub mod bentpipe;/d' emane-core/src/lib.rs

# tdma
mkdir -p plugins/tdma/src
cat << 'TOML' > plugins/tdma/Cargo.toml
[package]
name = "tdma"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
emane-core = { path = "../../emane-core" }
libc = "0.2"
TOML
mv emane-core/src/tdma/* plugins/tdma/src/ 2>/dev/null || true
sed -i '/pub mod tdma;/d' emane-core/src/lib.rs

# rfpipe
mkdir -p plugins/rfpipe/src
cat << 'TOML' > plugins/rfpipe/Cargo.toml
[package]
name = "rfpipe"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
emane-core = { path = "../../emane-core" }
libc = "0.2"
TOML
mv emane-core/src/rfpipe_mac.rs plugins/rfpipe/src/lib.rs 2>/dev/null || true
mv emane-core/src/rfpipe_message.rs plugins/rfpipe/src/ 2>/dev/null || true
sed -i '/pub mod rfpipe_mac;/d' emane-core/src/lib.rs
sed -i '/pub mod rfpipe_message;/d' emane-core/src/lib.rs

# Update Workspace
cat << 'TOML' > Cargo.toml
[workspace]
members = [
    "emane-core",
    "plugins/dummy-mac",
    "plugins/ieee80211abg",
    "plugins/bentpipe",
    "plugins/tdma",
    "plugins/rfpipe"
]
TOML
