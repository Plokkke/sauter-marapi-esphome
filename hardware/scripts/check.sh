#!/bin/sh
# Regenerates the MITM schematic, runs the ERC and exports the SVG next to it.
set -e
cd "$(dirname "$0")/.."
python3 scripts/generate-schematic.py
kicad-cli sch erc --severity-all --exit-code-violations -o linap-mitm-erc.rpt linap-mitm.kicad_sch
kicad-cli sch export svg -o . linap-mitm.kicad_sch >/dev/null 2>&1
echo "ERC clean, linap-mitm.svg exported"
