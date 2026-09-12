#!/usr/bin/env python3
"""Set IEEE 802.11abg data rates without requiring local plugin manifests."""

import argparse

from emane.shell.controlportclient import ControlPortClient


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("rate", type=int, help="IEEE rate index (for example, 10 for 36 Mbps)")
    parser.add_argument("--host", default="127.0.0.1")
    parser.add_argument("--port", default=47285, type=int)
    parser.add_argument("--nems", default=(1, 3), nargs="+", type=int)
    args = parser.parse_args()

    client = ControlPortClient(args.host, args.port)
    try:
        manifest = client.getManifest()
        for nem_id, components in sorted(manifest.items()):
            if nem_id not in args.nems:
                continue
            for build_id, layer, plugin in components:
                if layer == "MAC" and "ieee80211abg" in plugin:
                    updates = (
                        ("unicastrate", ControlPortClient.TYPE_UINT8, (args.rate,)),
                        ("multicastrate", ControlPortClient.TYPE_UINT8, (args.rate,)),
                    )
                    client.updateConfiguration(build_id, updates)
                    values = client.getConfiguration(
                        build_id, ("unicastrate", "multicastrate")
                    )
                    print(f"NEM {nem_id} MAC build {build_id}: {values}")
    finally:
        client.stop()


if __name__ == "__main__":
    main()
