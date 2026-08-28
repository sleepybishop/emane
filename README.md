emane
==

EMANE (Extendable Mobile Ad-hoc Network Emulator) is an open source framework 
which provides wireless network experimenters with a highly flexible modular 
environment for use during the design, development and testing of simple and 
complex network architectures. EMANE uses a physical layer model to account for 
signal propagation, antenna profile effects and interference sources in order 
to provide a realistic environment for wireless experimentation. Individual 
radio model plugins are used to emulate the lowest layers of a waveform and can 
be combined with existing Software Defined Radio (SDR) implementations to 
enable shared code emulation.

### Architecture

EMANE has been completely rewritten in **Rust** to provide memory safety, fearless concurrency, and massive performance improvements over the legacy C++ architecture. 
It uses a pure-Rust `cdylib` architecture for its plugins and natively handles packet routing, statistics, and RF spectrum monitoring.

### Build and install

```sh
make prepare-install
sudo make install
```

The default prefix is `/usr`. For a custom prefix, pass the same `PREFIX` to
both commands, for example `make prepare-install PREFIX=/opt/emane` followed by
`sudo make install PREFIX=/opt/emane`. See [INSTALL](INSTALL) for the installed
layout and staged packaging instructions. `cargo install` alone does not
install EMANE's model plugins, manifests, schemas, or Python tools.

Need more information?
==
Visit the EMANE Wiki:

 https://github.com/adjacentlink/emane/wiki
