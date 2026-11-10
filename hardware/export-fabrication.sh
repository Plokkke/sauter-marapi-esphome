#!/usr/bin/env bash
# Gerbers, drill, JLCPCB BOM and placement, LCSC cart, renders for ordering one board (KiCad 10 kicad-cli).
# Usage: export-fabrication.sh linap   (the board folder under hardware/)
set -euo pipefail

cd "$(dirname "$0")/$1"
BOARD=radiator-controller-$1
OUT=fabrication

rm -rf "$OUT" && mkdir -p "$OUT/gerbers"
kicad-cli pcb drc --schematic-parity --exit-code-violations --severity-error -o "$OUT/drc.rpt" "$BOARD.kicad_pcb"
kicad-cli pcb export gerbers --layers F.Cu,B.Cu,F.Paste,F.SilkS,B.SilkS,F.Mask,B.Mask,Edge.Cuts \
  --subtract-soldermask --no-protel-ext -o "$OUT/gerbers" "$BOARD.kicad_pcb"
kicad-cli pcb export drill --format excellon --excellon-separate-th --generate-map --map-format gerberx2 \
  -o "$OUT/gerbers" "$BOARD.kicad_pcb"
(cd "$OUT/gerbers" && zip -q "../$BOARD-gerbers.zip" ./*)
# JLCPCB places the parts of the CPL (SMD only); every other part with an LCSC number goes to the LCSC cart
kicad-cli sch export bom --fields 'Value,Reference,Footprint,LCSC,${QUANTITY}' \
  --labels "Comment,Designator,Footprint,LCSC Part #,Quantity" --group-by "LCSC,Value,Footprint" \
  --ref-range-delimiter "" --exclude-dnp -o "$OUT/$BOARD-bom.csv" "$BOARD.kicad_sch"
kicad-cli pcb export pos --format csv --units mm --side front --exclude-dnp --exclude-fp-th -o "$OUT/$BOARD-cpl.csv" "$BOARD.kicad_pcb"
sed -i.bak '1s/.*/Designator,Val,Package,Mid X,Mid Y,Rotation,Layer/' "$OUT/$BOARD-cpl.csv" && rm "$OUT/$BOARD-cpl.csv.bak"
../split-bom.py "$OUT/$BOARD-bom.csv" "$OUT/$BOARD-cpl.csv" "$OUT/$BOARD-bom-jlcpcb.csv" "$OUT/$BOARD-lcsc-cart.csv"

for side in top bottom; do
  kicad-cli pcb render --side "$side" --width 1600 --height 1000 -o "$BOARD-$side.png" "$BOARD.kicad_pcb"
done
kicad-cli pcb render --perspective --rotate "-45,0,-30" --zoom 1.3 --width 1600 --height 1000 -o "$BOARD-3d.png" "$BOARD.kicad_pcb"
kicad-cli pcb export step --force -o "$BOARD.step" "$BOARD.kicad_pcb"
# schematic render shown in the README
kicad-cli sch export svg --exclude-drawing-sheet -o . "$BOARD.kicad_sch"
