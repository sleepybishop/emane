#!/usr/bin/env python3
"""Validate a generated EMANE manifest directory."""

from pathlib import Path
import sys
import xml.etree.ElementTree as ET


EXPECTED = {
    "emanephy",
    "virtualtransport",
    "rawtransport",
    "rfpipemaclayer",
    "ieee80211abgmaclayer",
    "tdmaeventschedulerradiomodel",
    "bentpipemaclayer",
    "commeffectshim",
    "timinganalysisshim",
    "phyapitestshim",
    "bypassmaclayer",
    "bypassphylayer",
    "dummy-mac",
    "nemmanager",
    "transportmanager",
    "eventgeneratormanager",
    "eventagentmanager",
}


def main() -> int:
    if len(sys.argv) != 2:
        print(f"usage: {sys.argv[0]} MANIFEST-DIRECTORY", file=sys.stderr)
        return 2

    directory = Path(sys.argv[1])
    files = sorted(directory.glob("*.xml"))
    names: list[str] = []
    for path in files:
        root = ET.parse(path).getroot()
        if root.tag != "manifest":
            raise ValueError(f"{path}: root element is {root.tag!r}, not 'manifest'")
        plugin = root.find("plugin")
        name = plugin.get("name") if plugin is not None else None
        if not name:
            raise ValueError(f"{path}: manifest has no named plugin")
        names.append(name)

    found = set(names)
    duplicate = sorted(name for name in found if names.count(name) > 1)
    missing = sorted(EXPECTED - found)
    extra = sorted(found - EXPECTED)
    if duplicate or missing or extra or len(files) != len(EXPECTED):
        raise ValueError(
            f"manifest inventory mismatch: duplicate={duplicate}, "
            f"missing={missing}, extra={extra}, files={len(files)}"
        )

    print(f"validated {len(files)} manifests in {directory}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
