#!/bin/sh
set -eu

if [ "$#" -ne 1 ]; then
    echo "usage: $0 OUTPUT-DIRECTORY" >&2
    exit 2
fi

output_dir=$1
emaneinfo=${EMANEINFO:-emaneinfo}
schema=${EMANE_MANIFEST_SCHEMA:-}

mkdir -p "$output_dir"
work_dir=$(mktemp -d "${TMPDIR:-/tmp}/emane-manifests.XXXXXX")
trap 'rm -rf "$work_dir"' EXIT HUP INT TERM

plugins='emanephy
virtualtransport
rawtransport
rfpipemaclayer
ieee80211abgmaclayer
tdmaeventschedulerradiomodel
emane-model-bentpipe
commeffectshim
timinganalysisshim
phyapitestshim
bypassmaclayer
bypassphylayer
dummy_mac
nemmanager
transportmanager
eventgeneratormanager
eventagentmanager'

for plugin in $plugins; do
    "$emaneinfo" --manifest "$plugin" >"$work_dir/$plugin.xml"
done

if [ -n "$schema" ] && command -v xmllint >/dev/null 2>&1; then
    for manifest in "$work_dir"/*.xml; do
        xmllint --noout --schema "$schema" "$manifest"
    done
fi

# The output is a generated directory; remove stale manifests before replacing
# it so a renamed or removed plugin cannot survive unnoticed.
find "$output_dir" -maxdepth 1 -type f -name '*.xml' -delete
install -m 0644 "$work_dir"/*.xml "$output_dir/"
