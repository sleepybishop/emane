#!/bin/bash
if ! grep -q "pub mod ota_transmitter;" rust/emane-core/src/controls/mod.rs; then
    echo "pub mod ota_transmitter;" >> rust/emane-core/src/controls/mod.rs
fi
