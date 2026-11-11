#!/usr/bin/env python3
"""Generates the KiCad schematic of the discovery bench: one Wemos D1 mini on the Linap bus, used by every stage.

Usage: discovery/bench/generate-schematic.py   (then check with kicad-cli sch erc, export with kicad-cli sch export svg)
"""
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent
sys.path.insert(0, str(ROOT.parent.parent / "hardware" / "scripts"))
from kicad_sheet import (GREEN, GREY, ORANGE, RED, Sheet, junction, label, lib_symbol, no_connect,  # noqa: E402
                         project_files, rectangle, text, title_block, wire)
from modules import D1_MINI as D1  # noqa: E402

NAME = "linap-bench"
REVISION, DATE = "2", "2026-11-08"
LIBS = (("Device", "R"), ("Device", "Q_NPN"), ("Jumper", "Jumper_2_Open"),
        ("Connector_Generic", "Conn_01x02"), ("Connector_Generic", "Conn_01x03"), ("power", "GND"),
        ("power", "PWR_FLAG"))

FRAME_TITLE = "ON THE RADIATOR (STAGES 1 AND 3), EVERYTHING INSIDE THE RED FRAME IS AT MAINS POTENTIAL"
NOTES = [
    "Stage 1, sniffer (radiator): J2 taps the ribbon between both boards. J1 powers, JP1 open. Q1 stays off.",
    "Stage 2, power emulator (bench, NO MAINS): J2 to the command board alone. USB powers, J1 unplugged, JP1 closed.",
    "Stage 3, controller emulator (radiator): J2 to the power board alone. J1 powers, JP1 open.",
    "R7 (crossed: stage 3 only): the command board is the bus pull-up, so the power board alone needs R7.",
    "1. Flash over USB BEFORE wiring, then unplug it for good: never USB while J2 reaches the power board.",
    "2. Lab supply output must float: no earth terminal, no link between - and earth. Set 5.0 V, limit 300 mA.",
    "3. Wire with the breaker OFF. Once powered, touch nothing: read and drive over Wi-Fi only.",
    "Revision 1 (2026-11-07): listen only. Revision 2 (2026-11-08): Q1 + R6 transmit from D8, R7 optional pull-up.",
]

U1 = (114.3, 95.25)
NODE_X, J2 = 170.18, (236.22, 96.52)
DATA_X, R7_X, VCC_X = 205.74, 215.9, 220.98


def d1(pin):
    """Schematic position of a D1 mini pin, by its short name (D2, 5V...)."""
    return D1.pin(U1, pin)


def module(sheet, used):
    pins = [(name.split("/")[0], number) for side, _, _ in D1.sides() for name, number, _ in side]
    return [sheet.symbol(f"{NAME}:WEMOS_D1_mini", "U1", "Wemos D1 mini", U1, D1.numbers(),
                         ref_at=(U1[0], U1[1] - 16.51), value_at=(U1[0], U1[1] + 16.51),
                         footprint="RF_Module:WEMOS_D1_mini_light"),
            *[no_connect(d1(pin)) for pin, _ in pins if pin not in used]]


def supply(sheet):
    five_v, ground = d1("5V"), d1("GND")
    j1_gnd, j1_5v = (144.78, 110.49), (144.78, 113.03)
    return [
        *wire([ground, (139.7, ground[1]), (139.7, 106.68), (139.7, j1_gnd[1]), (142.24, j1_gnd[1]), j1_gnd], GREY),
        *wire([five_v, (134.62, five_v[1]), (134.62, j1_5v[1]), (139.7, j1_5v[1]), j1_5v], ORANGE),
        label("5V", (130.81, five_v[1])), junction((142.24, j1_gnd[1])), junction((139.7, j1_5v[1])),
        *wire([(139.7, 106.68), (137.16, 106.68)], GREY), junction((139.7, 106.68)),
        sheet.power("GND", (137.16, 106.68)),
        sheet.power("PWR_FLAG", (142.24, j1_gnd[1])), sheet.power("PWR_FLAG", (139.7, j1_5v[1]), rot=180),
        sheet.symbol("Connector_Generic:Conn_01x02", "J1", "Lab supply 5.0 V", (149.86, j1_gnd[1]), ["1", "2"],
                     ref_at=(149.86, 106.68), value_at=(149.86, 118.11)),
        text("1 = - (to G)", (153.67, 111.76)), text("2 = + (to 5V)", (153.67, 114.3)),
        text("floating, 300 mA limit", (142.24, 121.92)),
    ]


def divider(sheet, data_end):
    data_y = d1("D2")[1]
    node = (NODE_X, data_y)
    return [
        *wire([d1("D2"), node], GREEN), junction(node), label("GPIO4", (132.08, data_y)),
        sheet.resistor("R2", "15k", (NODE_X, data_y + 7.62), horizontal=False),
        *wire([node, (NODE_X, data_y + 3.81)], GREEN),
        *wire([(NODE_X, data_y + 11.43), (NODE_X, data_y + 13.97)], GREY), sheet.power("GND", (NODE_X, data_y + 13.97)),
        sheet.resistor("R1", "10k", (NODE_X + 12.7, data_y), horizontal=True),
        *wire([node, (NODE_X + 8.89, data_y)], GREEN), *wire([(NODE_X + 16.51, data_y), *data_end], GREEN),
        label("LINAP_DATA", (NODE_X + 19.05, data_y)),
        text("Divider: 5.2 V x 15k / 25k = 3.1 V on D2 (GPIO4)", (140.97, data_y - 10.16)),
    ]


def transmitter(sheet):
    data_y = d1("D2")[1]
    q1, d8 = (DATA_X - 2.54, data_y + 15.24), d1("D8")
    base, r6 = (q1[0] - 5.08, q1[1]), (q1[0] - 11.43, q1[1])
    return [
        *wire([d8, (d8[0] - 12.7, d8[1])]), label("LINAP_TX", (d8[0] - 12.7, d8[1])),
        *wire([(r6[0] - 3.81, q1[1]), (r6[0] - 16.51, q1[1])]), label("LINAP_TX", (r6[0] - 16.51, q1[1])),
        sheet.resistor("R6", "10k", r6, horizontal=True), *wire([(r6[0] + 3.81, q1[1]), base]),
        sheet.symbol("Device:Q_NPN", "Q1", "BC547", q1, ["B", "C", "E"],
                     ref_at=(q1[0] + 6.35, q1[1] - 2.54), value_at=(q1[0] + 8.89, q1[1] + 1.27)),
        *wire([(DATA_X, q1[1] - 5.08), (DATA_X, data_y)], GREEN), junction((DATA_X, data_y)),
        *wire([(DATA_X, q1[1] + 5.08), (DATA_X, q1[1] + 7.62)], GREY), sheet.power("GND", (DATA_X, q1[1] + 7.62)),
        text("TX: D8 high -> Q1 on -> DATA low (UART inverted)", (r6[0] - 15.24, q1[1] + 12.7)),
    ]


def pull_up(sheet):
    data_y, r7 = d1("D2")[1], (R7_X, d1("D2")[1] + 3.81)
    return [
        sheet.resistor("R7", "4.7k", r7, horizontal=False, dnp=True,
                       text_at=((R7_X - 5.08, r7[1] - 1.27), (R7_X - 5.08, r7[1] + 1.27))),
        junction((R7_X, data_y)), *wire([(R7_X, r7[1] + 3.81), (VCC_X, r7[1] + 3.81)], ORANGE),
        junction((VCC_X, r7[1] + 3.81)),
    ]


def ribbon(sheet):
    pin_x = J2[0] - 5.08
    data, vcc, ground = (pin_x, J2[1] - 2.54), (pin_x, J2[1]), (pin_x, J2[1] + 2.54)
    jp1 = (VCC_X, J2[1] + 10.16)
    return [
        sheet.symbol("Connector_Generic:Conn_01x03", "J2", "Linap ribbon", J2, ["1", "2", "3"],
                     ref_at=(J2[0], J2[1] - 6.35), value_at=(J2[0] + 10.16, J2[1] + 7.62)),
        text("1 white = DATA", (J2[0] + 2.54, data[1] + 1.27)), text("2 red = VCC", (J2[0] + 2.54, vcc[1] + 1.27)),
        text("3 black = GND", (J2[0] + 2.54, ground[1] + 1.27)),
        *wire([vcc, (VCC_X, vcc[1]), (VCC_X, jp1[1] - 5.08)], ORANGE),
        sheet.symbol("Jumper:Jumper_2_Open", "JP1", "VCC feed", jp1, ["1", "2"], rot=90,
                     ref_at=(VCC_X + 6.35, jp1[1] - 1.27), value_at=(VCC_X + 6.35, jp1[1] + 1.27)),
        *wire([(VCC_X, jp1[1] + 5.08), (VCC_X, jp1[1] + 7.62)], ORANGE), label("5V", (VCC_X, jp1[1] + 7.62)),
        *wire([ground, (pin_x - 5.08, ground[1]), (pin_x - 5.08, ground[1] + 2.54)], GREY),
        sheet.power("GND", (pin_x - 5.08, ground[1] + 2.54)),
    ], data


def frame():
    lines = [text(FRAME_TITLE, (58.42, 52.07), size=1.8, bold=True, color=RED), text("Notes", (58.42, 137.16), bold=True)]
    lines += [text(line, (58.42, 140.97 + 3.81 * n)) for n, line in enumerate(NOTES)]
    return [rectangle((55.88, 45.72), (287.02, 175.26), 0.6, "dash", RED), *lines]


def items(sheet):
    ribbon_items, data_pin = ribbon(sheet)
    data_end = [(DATA_X, data_pin[1]), (R7_X, data_pin[1]), data_pin]
    return [*module(sheet, {"D2", "D8", "5V", "GND"}), *supply(sheet), *divider(sheet, data_end),
            *transmitter(sheet), *pull_up(sheet), *ribbon_items, *frame()]


def main():
    sheet = Sheet(NAME)
    libs = D1.symbol(f"{NAME}:WEMOS_D1_mini") + "".join(lib_symbol(lib, name) for lib, name in LIBS)
    block = title_block("Linap bench - Wemos D1 mini on the Linap bus", DATE, REVISION,
                        "Sauter Marapi tactile - Linap bus discovery",
                        "Firmwares: discovery/<stage>/*.yaml - procedure and revisions: discovery/README.md")
    sheet.write(ROOT / f"{NAME}.kicad_sch", "A3", block, libs, items(sheet))
    project_files(ROOT, NAME, D1, "Wemos D1 mini drawn as the board")


main()
