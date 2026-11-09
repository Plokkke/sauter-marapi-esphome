#!/usr/bin/env python3
"""Split the KiCad BOM: parts JLCPCB can place (those in the CPL) and the LCSC cart of every part to buy.

The cart lists every part, the SMD ones included, so the board can be hand-soldered; when JLCPCB assembles the board,
disable the lines marked "JLCPCB" in the LCSC BOM Tool. Lines marked "in stock" are bought only when the stock runs out.

Usage: split-bom.py <bom.csv> <cpl.csv> <jlcpcb-bom.csv> <lcsc-cart.csv>
"""
import csv
import sys

# Bought with the board but not drawn on it: the fuse that goes into the F1 holder
EXTRA_PARTS = [{"LCSC Part #": "C142808", "Quantity": "1", "Comment": "F1 fuse TR5 T1A 250V (Littelfuse 37211000001)"}]
# Through-hole resistors already in the drawer: 220 and 1k
IN_STOCK = {"C3537012", "C1364484"}


def read(path):
    with open(path, newline="") as f:
        return list(csv.DictReader(f))


def write(path, fields, rows):
    with open(path, "w", newline="") as f:
        writer = csv.DictWriter(f, fields, extrasaction="ignore")
        writer.writeheader()
        writer.writerows(rows)


def cart_line(row, assembled):
    notes = [n for n, on in (("JLCPCB", row in assembled), ("in stock", row["LCSC Part #"] in IN_STOCK)) if on]
    comment = f'{row["Designator"]} {row["Comment"]}' + (f' ({", ".join(notes)})' if notes else "")
    return {**row, "Comment": comment}


def merged(rows):
    """One cart line per LCSC part, as the BOM Tool expects."""
    lines = {}
    for row in rows:
        line = lines.setdefault(row["LCSC Part #"], {"LCSC Part #": row["LCSC Part #"], "Quantity": 0, "Comment": []})
        line["Quantity"] += int(row["Quantity"])
        line["Comment"].append(row["Comment"])
    return [{**line, "Comment": " / ".join(line["Comment"])} for line in lines.values()]


def main(bom_path, cpl_path, jlcpcb_path, lcsc_path):
    bom = [row for row in read(bom_path) if row["LCSC Part #"]]
    placed = {row["Designator"] for row in read(cpl_path)}
    assembled = [row for row in bom if set(row["Designator"].split(",")) <= placed]
    write(jlcpcb_path, ["Comment", "Designator", "Footprint", "LCSC Part #"], assembled)
    write(lcsc_path, ["LCSC Part #", "Quantity", "Comment"], merged([cart_line(r, assembled) for r in bom] + EXTRA_PARTS))


if __name__ == "__main__":
    main(*sys.argv[1:5])
