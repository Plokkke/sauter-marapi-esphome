#!/usr/bin/env python3
"""Generates the KiCad schematic of the MITM board: a Wemos D1 mini cutting the Linap DATA line in two.

Usage: hardware/scripts/generate-schematic.py   (then hardware/scripts/check.sh: ERC and SVG export)
Rules: CLAUDE.md. Firmware: firmware/sauter-marapi.yaml (pins in its header).
"""
from dataclasses import dataclass, replace
from pathlib import Path

from kicad_sheet import (BROWN, GREEN, GREY, ORANGE, RED, TEAL, Sheet, block, junction, label, lib_symbol,
                         project_files, rectangle, text, title_block, wire)
from modules import D1_MINI

ROOT = Path(__file__).resolve().parent.parent
NAME = "linap-mitm"
REVISION, DATE = "3", "2026-11-11"
LIBS = (("Device", "R"), ("Device", "C_Polarized"), ("Transistor_BJT", "Q_NPN_EBC"),
        ("Connector_Generic", "Conn_01x03"), ("Connector_Generic", "Conn_01x08"), ("power", "GND"),
        ("power", "+5V"), ("power", "PWR_FLAG"))

RESISTOR = "Resistor_THT:R_Axial_DIN0207_L6.3mm_D2.5mm_P7.62mm_Horizontal"
LCSC_10K, LCSC_4K7 = "C57436", "C1364279"
# RX divider: 100k / 150k, so as not to load DATA (see the notes)
LCSC_100K, LCSC_150K = "C172965", "C119513"
RESISTOR_1W = "Resistor_THT:R_Axial_DIN0309_L9.0mm_D3.2mm_P12.70mm_Horizontal"
NPN, LCSC_NPN, TO92 = "2N3904", "C2874602", "Package_TO_SOT_THT:TO-92_Inline"
SOCKET, LCSC_SOCKET = "Connector_PinSocket_2.54mm:PinSocket_1x08_P2.54mm_Vertical", "C27438"
RIBBON = "Connector_PinHeader_2.54mm:PinHeader_1x03_P2.54mm_Vertical"

D1 = replace(D1_MINI, reference="MOD")
MOD1 = (66.04, 55.88)
SOCKET_OFFSET = 1.27

FRAME_TITLE = "ON THE RADIATOR THE WHOLE BOARD IS AT MAINS POTENTIAL (Linap GND tied to neutral): no USB, no earth"
NOTES = [
    "MOD1: Wemos D1 mini (ESP8266); its PCB antenna",
    "holds the link where the C3 super mini dropped it.",
    "Flash over USB BEFORE wiring. Never USB on the",
    "radiator: the whole board is at mains potential.",
    "Wire with the breaker OFF, lab supply floating.",
    "R5: DATA pull-up on the power board side; the",
    "command board pulls its own side up.",
    "R4, R9 hold the NPNs off while the D1 boots; D8",
    "(GPIO15) must stay low at boot: R3 + R4 and the",
    "D1's own 10 k keep it there. D3/D4 must be high",
    "at boot: they cannot drive an NPN.",
    "Q1, Q2: 2N3904 TO-92, E-B-C, flat side to you.",
    "R1/R2, R6/R7 load DATA: the command board pull-up",
    "(~6 k) fails at 25 k, is marginal at 50 k: 250 k.",
    "No internal pull-up on D5/D7 (firmware).",
    "J1, J2: ribbon connectors still to choose (the",
    "original ones if possible), LCSC TBD.",
    "Firmware: firmware/ (ESPHome). Power side: UART0",
    "swapped to D7/D8, TX inverted; command side:",
    "receiver on D5, bit-banged TX on D6.",
    "Wires: orange +5 V, grey GND, magenta 3.3 V,",
    "green TX, teal RX, brown Linap bus.",
    "Rev. 2 (2026-11-11): RX dividers 100k/150k.",
    "Rev. 3 (2026-11-11): Wemos D1 mini in place of",
    "the ESP32-C3 super mini.",
]


@dataclass
class Channel:
    """One half of DATA: its connector, divider (RX), NPN (TX) and optional pull-up."""
    title: str
    origin: tuple
    connector: tuple  # reference, value
    resistors: tuple  # divider top, divider bottom, base, base pull-down, DATA pull-up (or None)
    transistor: str
    data: str
    rx: str
    tx: str


POWER = Channel("Power board side: J1, RX R1/R2 (D7), TX Q1 (D8), pull-up R5",
                (116.84, 27.94), ("J1", "Ribbon to the power board"), ("R1", "R2", "R3", "R4", "R5"), "Q1",
                "LINAP_DATA_P", "D7_RX_P", "D8_TX_P")
COMMAND = Channel("Command board side: J2, RX R6/R7 (D5), TX Q2 (D6)",
                  (116.84, 93.98), ("J2", "Ribbon to the command board"), ("R6", "R7", "R8", "R9", None), "Q2",
                  "LINAP_DATA_C", "D5_RX_C", "D6_TX_C")


def module(sheet):
    pin = lambda name: D1.pin(MOD1, name)  # noqa: E731
    top = round(MOD1[1] - D1.top_y, 2)
    sockets_y = round(top + 7.62, 2)
    left_x, right_x = pin("RST")[0], pin("5V")[0]
    j3, j4 = (left_x - SOCKET_OFFSET, sockets_y), (right_x + SOCKET_OFFSET, sockets_y)
    out_left, out_right = j3[0] - 5.08, j4[0] + 5.08
    items = [*block((17.78, 27.94), (109.22, 78.74),
                    "Wemos D1 mini (MOD1), plugged into J3 (pins 1-8) and J4 (pins 16-9)"),
             sheet.symbol(f"{NAME}:{D1.name}", "MOD1", "Wemos D1 mini", MOD1, D1.numbers(),
                          ref_at=(MOD1[0], MOD1[1] - 19.05), value_at=(MOD1[0], MOD1[1] - 16.51), on_board=False),
             *socket(sheet, "J3", "Socket 1x8 (D1 pins 1-8)", j3, "", (out_left + 1.27, 72.39)),
             *socket(sheet, "J4", "Socket 1x8 (D1 pins 16-9)", j4, "y", (out_right - 3.81, 72.39))]
    for side, x, _ in D1.sides():
        for name, _, _ in side:
            y = pin(name.split("/")[0])[1]
            items += wire([(x + MOD1[0], y), (out_right if x > 0 else out_left, y)])
    return items + module_outputs(sheet, out_right, out_left)


def socket(sheet, ref, value, at, mirror, text_at):
    return [sheet.symbol("Connector_Generic:Conn_01x08", ref, value, at, [str(n) for n in range(1, 9)],
                         mirror=mirror, ref_at=(text_at[0], text_at[1] - 2.54), value_at=text_at,
                         footprint=SOCKET, lcsc=LCSC_SOCKET)]


def module_outputs(sheet, out_right, out_left):
    y = lambda name: D1.pin(MOD1, name)[1]  # noqa: E731
    five_v_x = out_right + 7.62
    return [
        *wire([(out_right, y("5V")), (five_v_x, y("5V"))], ORANGE), sheet.power("+5V", (five_v_x, y("5V"))),
        sheet.power("GND", (out_right, y("GND")), rot=90),
        *[item for pin, name, colour in (("D5", COMMAND.rx, TEAL), ("D6", COMMAND.tx, GREEN),
                                         ("D7", POWER.rx, TEAL), ("D8", POWER.tx, GREEN))
          for item in (*wire([(out_left, y(pin)), (out_left - 5.08, y(pin))], colour),
                       label(name, (out_left - 5.08, y(pin)), angle=180))],
    ]


def supply(sheet):
    c1 = (45.72, 101.6)
    plus, minus = (c1[0], c1[1] - 3.81), (c1[0], c1[1] + 3.81)
    return [
        *block((17.78, 83.82), (109.22, 114.3), "Supply: ribbon 5 V from J1 to J2, bulk capacitor C1"),
        sheet.symbol("Device:C_Polarized", "C1", "470uF 10V", c1, ["1", "2"], ref_at=(c1[0] + 7.62, c1[1] - 1.27),
                     value_at=(c1[0] + 9.53, c1[1] + 1.27), footprint="Capacitor_THT:CP_Radial_D6.3mm_P2.50mm",
                     lcsc="C43326"),
        *wire([plus, (c1[0], 93.98)], ORANGE), sheet.power("+5V", (c1[0], 93.98)),
        *wire([(c1[0], 95.25), (35.56, 95.25)], ORANGE), junction((c1[0], 95.25)),
        sheet.power("PWR_FLAG", (35.56, 95.25)),
        *wire([minus, (c1[0], 109.22)], GREY), sheet.power("GND", (c1[0], 109.22)),
        *wire([(c1[0], 106.68), (35.56, 106.68)], GREY), junction((c1[0], 106.68)),
        sheet.power("PWR_FLAG", (35.56, 106.68), rot=180),
        text("C1 next to the D1 5 V pin:", (62.23, 96.52)), text("takes the Wi-Fi bursts (the ribbon", (62.23, 100.33)),
        text("gives ~160 mA, command board included).", (62.23, 104.14)),
        text("VCC: straight from J1 pin 2 to J2 pin 2.", (62.23, 107.95)),
    ]


def channel(sheet, ch):
    ox, oy = ch.origin
    data_y, jx, jy = oy + 22.86, ox + 71.12, oy + 35.56
    tap, pull_x = ox + 45.72, ox + 55.88
    top, bottom, base, pull_down, pull_up = ch.resistors
    items = [*block((ox, oy), (ox + 91.44, oy + 60.96), ch.title),
             *wire([(jx - 5.08, jy - 2.54), (ox + 63.5, jy - 2.54), (ox + 63.5, data_y), (ox + 31.75, data_y)], BROWN),
             junction((tap, data_y)), label(ch.data, (ox + 33.02, data_y))]
    items += divider(sheet, ch, ox, data_y, top, bottom)
    items += transmitter(sheet, ch, ox, oy, tap, data_y, base, pull_down)
    if pull_up:
        items += [sheet.resistor(pull_up, "4.7k", (pull_x, oy + 15.24), horizontal=False, footprint=RESISTOR,
                                 lcsc=LCSC_4K7),
                  *wire([(pull_x, oy + 19.05), (pull_x, data_y)], BROWN), junction((pull_x, data_y)),
                  sheet.power("+5V", (pull_x, oy + 11.43))]
    return items + connector(sheet, ch, jx, jy) + [
        text("RX: 5.2 V x 150k / 250k = 3.1 V. TX: GPIO high -> NPN on -> DATA low.", (ox + 2.54, oy + 59.69))]


def divider(sheet, ch, ox, data_y, top, bottom):
    node = (ox + 17.78, data_y)
    return [
        sheet.resistor(top, "100k", (ox + 27.94, data_y), horizontal=True, footprint=RESISTOR, lcsc=LCSC_100K),
        *wire([(ox + 24.13, data_y), node, (ox + 5.08, data_y)], TEAL), junction(node),
        label(ch.rx, (ox + 5.08, data_y)),
        sheet.resistor(bottom, "150k", (node[0], data_y + 7.62), horizontal=False, footprint=RESISTOR_1W,
                       lcsc=LCSC_150K),
        *wire([node, (node[0], data_y + 3.81)], TEAL), sheet.power("GND", (node[0], data_y + 11.43)),
    ]


def transmitter(sheet, ch, ox, oy, tap, data_y, base, pull_down):
    q, base_node = (tap - 2.54, oy + 43.18), (ox + 36.83, oy + 43.18)
    return [
        sheet.symbol("Transistor_BJT:Q_NPN_EBC", ch.transistor, NPN, q, ["1", "2", "3"],
                     ref_at=(q[0] + 6.35, q[1] - 1.27), value_at=(q[0] + 6.35, q[1] + 1.27), footprint=TO92,
                     lcsc=LCSC_NPN),
        *wire([(tap, q[1] - 5.08), (tap, data_y)], BROWN), sheet.power("GND", (tap, q[1] + 5.08)),
        sheet.resistor(base, "10k", (ox + 31.75, q[1]), horizontal=True, footprint=RESISTOR, lcsc=LCSC_10K),
        *wire([(ox + 27.94, q[1]), (ox + 5.08, q[1])], GREEN), label(ch.tx, (ox + 5.08, q[1])),
        *wire([(ox + 35.56, q[1]), (q[0] - 5.08, q[1])], GREEN), junction(base_node),
        *wire([base_node, (base_node[0], q[1] + 3.81)], GREEN),
        sheet.resistor(pull_down, "10k", (base_node[0], q[1] + 7.62), horizontal=False, footprint=RESISTOR,
                       lcsc=LCSC_10K, text_at=((base_node[0] - 6.35, q[1] + 6.35), (base_node[0] - 6.35, q[1] + 8.89))),
        sheet.power("GND", (base_node[0], q[1] + 11.43)),
    ]


def connector(sheet, ch, jx, jy):
    ref, value = ch.connector
    pins_x = jx - 5.08
    return [
        sheet.symbol("Connector_Generic:Conn_01x03", ref, value, (jx, jy), ["1", "2", "3"],
                     ref_at=(jx, jy - 6.35), value_at=(jx - 2.54, jy + 7.62), footprint=RIBBON, lcsc="TBD"),
        text("1 white DATA", (jx + 2.54, jy - 1.27)), text("2 red VCC", (jx + 2.54, jy + 1.27)),
        text("3 black GND", (jx + 2.54, jy + 3.81)),
        *wire([(pins_x, jy), (pins_x - 7.62, jy)], ORANGE), sheet.power("+5V", (pins_x - 7.62, jy)),
        *wire([(pins_x, jy + 2.54), (pins_x - 5.08, jy + 2.54)], GREY), sheet.power("GND", (pins_x - 5.08, jy + 2.54)),
    ]


def notes():
    x, y = 213.36, 33.02
    return [*block((210.82, 27.94), (281.94, 154.94), "Notes"),
            *[text(line, (x, y + 3.81 * (n + 1))) for n, line in enumerate(NOTES)]]


def frame():
    return [rectangle((12.7, 17.78), (284.48, 160.02), 0.6, "dash", RED),
            text(FRAME_TITLE, (15.24, 24.13), size=1.5, bold=True, color=RED)]


def main():
    sheet = Sheet(NAME)
    libs = D1.symbol(f"{NAME}:{D1.name}") + "".join(lib_symbol(lib, name) for lib, name in LIBS)
    items = [*frame(), *module(sheet), *supply(sheet), *channel(sheet, POWER), *channel(sheet, COMMAND), *notes()]
    block_ = title_block("Linap MITM - Wemos D1 mini between the two boards", DATE, REVISION,
                         "Sauter Marapi tactile - ESPHome on Wemos D1 mini",
                         "DANGER: on the radiator the whole board is at mains potential (Linap GND tied to neutral)")
    sheet.write(ROOT / f"{NAME}.kicad_sch", "A4", block_, libs, items)
    project_files(ROOT, NAME, D1, "Wemos D1 mini drawn as the board")


main()
