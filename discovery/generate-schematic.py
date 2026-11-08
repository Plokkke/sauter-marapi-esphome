#!/usr/bin/env python3
"""Generates the KiCad schematic of the discovery bench: one Wemos D1 mini on the Linap bus, used by every stage.

Usage: discovery/generate-schematic.py   (then check with kicad-cli sch erc, export with kicad-cli sch export svg)
"""
import uuid
from pathlib import Path

ROOT = Path(__file__).resolve().parent
NAME = "linap-bench"
REVISION, DATE = "2", "2026-11-08"
KICAD_SYMBOLS = Path("/Applications/KiCad/KiCad.app/Contents/SharedSupport/symbols")
FONT = "(effects (font (size 1.27 1.27)))"
HIDDEN = "(effects (font (size 1.27 1.27)) (hide yes))"
RED, GREY, GREEN, ORANGE = "(color 200 0 0 1)", "(color 90 90 90 1)", "(color 0 150 70 1)", "(color 230 120 0 1)"
LIBS = (("Device", "R"), ("Device", "Q_NPN"), ("Jumper", "Jumper_2_Open"),
        ("Connector_Generic", "Conn_01x02"), ("Connector_Generic", "Conn_01x03"), ("power", "GND"),
        ("power", "PWR_FLAG"))

FRAME_TITLE = "ON THE RADIATOR (STAGES 1 AND 3), EVERYTHING INSIDE THE RED FRAME IS AT MAINS POTENTIAL"
NOTES = [
    "Stage 1, sniffer (radiator): J2 taps the ribbon between both boards. J1 powers, JP1 open. Q1 stays off.",
    "Stage 2, power emulator (bench, NO MAINS): J2 to the command board alone. USB powers, J1 unplugged, JP1 closed.",
    "Stage 3, controller emulator (radiator): J2 to the power board alone. J1 powers, JP1 open.",
    "R7 (not fitted): stage 3 only, if the heartbeat log reads DATA low at idle.",
    "1. Flash over USB BEFORE wiring, then unplug it for good: never USB while J2 reaches the power board.",
    "2. Lab supply output must float: no earth terminal, no link between - and earth. Set 5.0 V, limit 300 mA.",
    "3. Wire with the breaker OFF. Once powered, touch nothing: read and drive over Wi-Fi only.",
    "Revision 1 (2026-11-07): listen only. Revision 2 (2026-11-08): Q1 + R6 transmit from D8, R7 optional pull-up.",
]

U1 = (114.3, 95.25)
# D1 mini pins in their physical place, top view, antenna up, USB down: (name, number, electrical type)
D1_LEFT = (("RST", "1", "input"), ("A0", "2", "input"), ("D0/GPIO16", "3", "bidirectional"),
           ("D5/GPIO14", "4", "bidirectional"), ("D6/GPIO12", "5", "bidirectional"),
           ("D7/GPIO13", "6", "bidirectional"), ("D8/GPIO15", "7", "bidirectional"), ("3V3", "8", "power_out"))
D1_RIGHT = (("TX/GPIO1", "16", "bidirectional"), ("RX/GPIO3", "15", "bidirectional"),
            ("D1/GPIO5", "14", "bidirectional"), ("D2/GPIO4", "13", "bidirectional"),
            ("D3/GPIO0", "12", "bidirectional"), ("D4/GPIO2", "11", "bidirectional"), ("GND", "10", "power_in"),
            ("5V", "9", "power_in"))
D1_SIDES = ((D1_LEFT, -15.24, 0), (D1_RIGHT, 15.24, 180))
NODE_X, J2 = 170.18, (236.22, 96.52)
DATA_X, R7_X, VCC_X = 205.74, 215.9, 220.98


def u():
    return str(uuid.uuid4())


def d1_symbol(name):
    """Wemos D1 mini drawn as the board: the two 1x8 rows in their physical order."""
    pins = [f'(pin {kind} line (at {x} {round(8.89 - 2.54 * n, 2)} {angle}) (length 2.54) '
            f'(name "{pin}" {FONT}) (number "{number}" {FONT}))'
            for side, x, angle in D1_SIDES for n, (pin, number, kind) in enumerate(side)]
    body = ('(symbol "WEMOS_D1_mini_0_1" (rectangle (start -12.7 13.97) (end 12.7 -13.97) '
            '(stroke (width 0.254) (type default)) (fill (type background))) '
            f'(text "ANTENNA" (at 0 11.43 0) {FONT}) (text "USB" (at 0 -11.43 0) {FONT}))')
    props = [prop("Reference", "U", (0, 16.51)), prop("Value", "WEMOS_D1_mini", (0, -16.51)),
             prop("Footprint", "RF_Module:WEMOS_D1_mini_light", (0, 0), True),
             prop("Datasheet", "https://www.wemos.cc/en/latest/d1/d1_mini.html", (0, 0), True),
             prop("Description", "Wemos D1 mini (ESP8266), pins in their physical place, top view, USB down",
                  (0, 0), True)]
    return (f'\t(symbol "{name}" (exclude_from_sim no) (in_bom yes) (on_board yes) {" ".join(props)} {body} '
            f'(symbol "WEMOS_D1_mini_1_1" {" ".join(pins)}))\n')


def lib_symbol(lib, name):
    text = (KICAD_SYMBOLS / f"{lib}.kicad_sym").read_text()
    start = text.index(f'\n\t(symbol "{name}"\n') + 1
    end = text.index("\n\t)\n", start) + 3
    block = text[start:end].replace(f'(symbol "{name}"', f'(symbol "{lib}:{name}"', 1)
    return "".join("\t" + line for line in block.splitlines(keepends=True))


def prop(name, value, at, hidden=False, angle=0):
    return f'(property "{name}" "{value}" (at {at[0]} {at[1]} {angle}) {HIDDEN if hidden else FONT})'


class Sheet:
    """The schematic being written: its uuid and power symbol counter."""

    def __init__(self):
        self.uuid, self.power_count = u(), 0

    def symbol(self, lib_id, ref, value, at, pins, rot=0, ref_at=None, value_at=None, power=False,
               footprint="", value_hidden=False, dnp=False):
        x, y = at
        props = [prop("Reference", ref, ref_at or (x, y - 5.08), hidden=power, angle=rot),
                 prop("Value", value, value_at or (x, y + 5.08), hidden=value_hidden, angle=rot),
                 prop("Footprint", footprint, at, True), prop("Datasheet", "", at, True),
                 prop("Description", "", at, True)]
        head = (f'(symbol (lib_id "{lib_id}") (at {x} {y} {rot}) (unit 1) '
                f'(body_style 1) (exclude_from_sim no) (in_bom {"no" if power else "yes"}) (on_board yes) '
                f'(in_pos_files yes) (dnp {"yes" if dnp else "no"}) (uuid "{u()}")')
        pin_part = " ".join(f'(pin "{p}" (uuid "{u()}"))' for p in pins)
        inst = f'(instances (project "{NAME}" (path "/{self.uuid}" (reference "{ref}") (unit 1))))'
        return f"\t{head} {' '.join(props)} {pin_part} {inst})"

    def power(self, name, at, rot=0):
        self.power_count += 1
        flag = name == "PWR_FLAG"
        return self.symbol(f"power:{name}", f"#FLG{self.power_count:02}" if flag else f"#PWR{self.power_count:02}",
                           name, at, ["1"], rot=rot, power=True, value_hidden=True)

    def resistor(self, ref, value, at, horizontal, dnp=False, text_at=None):
        x, y = at
        text_at = text_at or (((x, y - 3.81), (x, y + 3.81)) if horizontal
                              else ((x + 2.54, y - 1.27), (x + 2.54, y + 1.27)))
        return self.symbol("Device:R", ref, value, at, ["1", "2"], rot=90 if horizontal else 0,
                           ref_at=text_at[0], value_at=text_at[1], dnp=dnp)


def wire(points, color=""):
    stroke = f"(stroke (width 0.25) (type default) {color})" if color else "(stroke (width 0.25) (type default))"
    return [f'\t(wire (pts (xy {a[0]} {a[1]}) (xy {b[0]} {b[1]})) {stroke} (uuid "{u()}"))'
            for a, b in zip(points, points[1:])]


def text(value, at, size=1.27, bold=False, color=""):
    font = f"(font (size {size} {size}){' (bold yes)' if bold else ''}{' ' + color if color else ''})"
    return (f'\t(text "{value}" (exclude_from_sim no) (at {at[0]} {at[1]} 0) (effects {font} '
            f'(justify left bottom)) (uuid "{u()}"))')


def label(name, at):
    return f'\t(label "{name}" (at {at[0]} {at[1]} 0) {FONT[:-1]} (justify left bottom)) (uuid "{u()}"))'


def no_connect(at):
    return f'\t(no_connect (at {at[0]} {at[1]}) (uuid "{u()}"))'


def junction(at):
    return f'\t(junction (at {at[0]} {at[1]}) (diameter 0) (color 0 0 0 0) (uuid "{u()}"))'


def d1(pin):
    """Schematic position of a D1 mini pin, by its short name (D2, 5V...)."""
    for side, x, _ in D1_SIDES:
        names = [name.split("/")[0] for name, _, _ in side]
        if pin in names:
            return round(U1[0] + x, 2), round(U1[1] - 8.89 + 2.54 * names.index(pin), 2)
    raise KeyError(pin)


def module(sheet, used):
    pins = [(name.split("/")[0], number) for side, _, _ in D1_SIDES for name, number, _ in side]
    return [sheet.symbol(f"{NAME}:WEMOS_D1_mini", "U1", "Wemos D1 mini", U1, [number for _, number in pins],
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
    return [f'\t(rectangle (start 55.88 45.72) (end 287.02 175.26) (stroke (width 0.6) (type dash) {RED}) '
            f'(fill (type none)) (uuid "{u()}"))', *lines]


def items(sheet):
    ribbon_items, data_pin = ribbon(sheet)
    data_end = [(DATA_X, data_pin[1]), (R7_X, data_pin[1]), data_pin]
    return [*module(sheet, {"D2", "D8", "5V", "GND"}), *supply(sheet), *divider(sheet, data_end),
            *transmitter(sheet), *pull_up(sheet), *ribbon_items, *frame()]


def main():
    sheet = Sheet()
    libs = d1_symbol(f"{NAME}:WEMOS_D1_mini") + "".join(lib_symbol(lib, name) for lib, name in LIBS)
    body = "\n".join(items(sheet))
    (ROOT / f"{NAME}.kicad_sch").write_text(f'''(kicad_sch
\t(version 20260306)
\t(generator "eeschema")
\t(generator_version "10.0")
\t(uuid "{sheet.uuid}")
\t(paper "A3")
\t(title_block (title "Linap bench - Wemos D1 mini on the Linap bus") (date "{DATE}") (rev "{REVISION}") (company "Plokkke") (comment 1 "Sauter Marapi tactile - Linap bus discovery") (comment 2 "Firmwares: discovery/<stage>/*.yaml - procedure and revisions: discovery/README.md"))
\t(lib_symbols
{libs}\t)
{body}
\t(sheet_instances (path "/" (page "1")))
\t(embedded_fonts no)
)
''')
    (ROOT / f"{NAME}.kicad_sym").write_text(
        f'(kicad_symbol_lib\n\t(version 20241209)\n\t(generator "{NAME}")\n{d1_symbol("WEMOS_D1_mini")})\n')
    (ROOT / "sym-lib-table").write_text(
        f'(sym_lib_table\n  (version 7)\n  (lib (name "{NAME}")(type "KiCad")(uri "${{KIPRJMOD}}/{NAME}.kicad_sym")'
        '(options "")(descr "Wemos D1 mini drawn as the board"))\n)\n')
    (ROOT / f"{NAME}.kicad_pro").write_text(f'{{\n  "meta": {{\n    "filename": "{NAME}.kicad_pro",\n    "version": 3\n  }}\n}}\n')


main()
