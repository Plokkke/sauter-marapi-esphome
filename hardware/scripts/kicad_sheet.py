"""Writing KiCad 10 schematics from Python: shared by discovery/bench/ and hardware/ generators.

Coordinates are in mm, on the 1.27 mm grid; y grows downwards on the sheet, upwards inside a symbol.
"""
import uuid
from dataclasses import dataclass
from pathlib import Path

KICAD_SYMBOLS = Path("/Applications/KiCad/KiCad.app/Contents/SharedSupport/symbols")
FONT = "(effects (font (size 1.27 1.27)))"
HIDDEN = "(effects (font (size 1.27 1.27)) (hide yes))"

# Wire colours (CLAUDE.md): the legend is written on every sheet
RED, GREY, GREEN, ORANGE = "(color 200 0 0 1)", "(color 90 90 90 1)", "(color 0 150 70 1)", "(color 230 120 0 1)"
TEAL, BROWN, MAGENTA = "(color 0 140 140 1)", "(color 140 80 30 1)", "(color 200 0 200 1)"
BLOCK_GREY = "(color 110 110 110 1)"
COLOUR_LEGEND = ("Wire colours: orange = +5 V, grey = GND, magenta = 3.3 V, green = TX, teal = RX, "
                 "brown = Linap bus")


def u():
    return str(uuid.uuid4())


def prop(name, value, at, hidden=False, angle=0):
    return f'(property "{name}" "{value}" (at {at[0]} {at[1]} {angle}) {HIDDEN if hidden else FONT})'


def lib_symbol(lib, name):
    """A symbol of the KiCad library, renamed lib:name as schematics embed it."""
    text = (KICAD_SYMBOLS / f"{lib}.kicad_sym").read_text()
    start = text.index(f'\n\t(symbol "{name}"\n') + 1
    end = text.index("\n\t)\n", start) + 3
    block = text[start:end].replace(f'(symbol "{name}"', f'(symbol "{lib}:{name}"', 1)
    return "".join("\t" + line for line in block.splitlines(keepends=True))


@dataclass
class Module:
    """A plug-in board drawn as itself: two rows of pins in their physical place, top view.

    Each row is a list of (name, number, electrical type) from top to bottom.
    """
    name: str
    description: str
    datasheet: str
    left: tuple
    right: tuple
    pin_x: float
    top_y: float
    body: tuple  # (half width, top, bottom) of the outline
    top_text: str
    bottom_text: str
    pin_length: float = 2.54
    reference: str = "MOD"
    footprint: str = ""

    def sides(self):
        return ((self.left, -self.pin_x, 0), (self.right, self.pin_x, 180))

    def symbol(self, lib_name):
        pins = [f'(pin {kind} line (at {x} {round(self.top_y - 2.54 * n, 2)} {angle}) (length {self.pin_length}) '
                f'(name "{pin}" {FONT}) (number "{number}" {FONT}))'
                for side, x, angle in self.sides() for n, (pin, number, kind) in enumerate(side)]
        half, top, bottom = self.body
        outline = (f'(symbol "{self.name}_0_1" (rectangle (start {-half} {top}) (end {half} {bottom}) '
                   '(stroke (width 0.254) (type default)) (fill (type background))) '
                   f'(text "{self.top_text}" (at 0 {top - 2.54} 0) {FONT}) '
                   f'(text "{self.bottom_text}" (at 0 {bottom + 2.54} 0) {FONT}))')
        props = [prop("Reference", self.reference, (0, top + 2.54)), prop("Value", self.name, (0, bottom - 2.54)),
                 prop("Footprint", self.footprint, (0, 0), True), prop("Datasheet", self.datasheet, (0, 0), True),
                 prop("Description", self.description, (0, 0), True)]
        return (f'\t(symbol "{lib_name}" (exclude_from_sim no) (in_bom yes) (on_board yes) {" ".join(props)} '
                f'{outline} (symbol "{self.name}_1_1" {" ".join(pins)}))\n')

    def pin(self, at, short_name):
        """Sheet position of a pin, by the part of its name before "/" (GPIO4, 5V...)."""
        for side, x, _ in self.sides():
            names = [name.split("/")[0] for name, _, _ in side]
            if short_name in names:
                return round(at[0] + x, 2), round(at[1] - self.top_y + 2.54 * names.index(short_name), 2)
        raise KeyError(short_name)

    def numbers(self):
        return [number for side, _, _ in self.sides() for _, number, _ in side]


class Sheet:
    """The schematic being written: its project, uuid and power symbol counter."""

    def __init__(self, project):
        self.project, self.uuid, self.power_count = project, u(), 0

    def symbol(self, lib_id, ref, value, at, pins, rot=0, ref_at=None, value_at=None, power=False,
               footprint="", value_hidden=False, dnp=False, mirror="", lcsc=None, on_board=True, text_angle=None):
        x, y = at
        angle = rot if text_angle is None else text_angle
        props = [prop("Reference", ref, ref_at or (x, y - 5.08), hidden=power, angle=angle),
                 prop("Value", value, value_at or (x, y + 5.08), hidden=value_hidden, angle=angle),
                 prop("Footprint", footprint, at, True), prop("Datasheet", "", at, True),
                 prop("Description", "", at, True)]
        if lcsc:
            props.append(prop("LCSC", lcsc, at, True))
        in_bom = "no" if power or not on_board else "yes"
        head = (f'(symbol (lib_id "{lib_id}") (at {x} {y} {rot}){f" (mirror {mirror})" if mirror else ""} (unit 1) '
                f'(body_style 1) (exclude_from_sim no) (in_bom {in_bom}) (on_board {"yes" if on_board else "no"}) '
                f'(in_pos_files yes) (dnp {"yes" if dnp else "no"}) (uuid "{u()}")')
        pin_part = " ".join(f'(pin "{p}" (uuid "{u()}"))' for p in pins)
        inst = f'(instances (project "{self.project}" (path "/{self.uuid}" (reference "{ref}") (unit 1))))'
        return f"\t{head} {' '.join(props)} {pin_part} {inst})"

    def power(self, name, at, rot=0):
        """Power symbol; a visible value (+5V) is written beyond its arrow, which points up (rot 0) or down (180)."""
        self.power_count += 1
        if name in ("GND", "PWR_FLAG"):
            ref = f"#FLG{self.power_count:02}" if name == "PWR_FLAG" else f"#PWR{self.power_count:02}"
            return self.symbol(f"power:{name}", ref, name, at, ["1"], rot=rot, power=True, value_hidden=True)
        x, y = at
        beyond = (x, y - 3.81) if rot == 0 else (x, y + 5.08)
        return self.symbol(f"power:{name}", f"#PWR{self.power_count:02}", name, at, ["1"], rot=rot, power=True,
                           value_at=beyond, text_angle=0)

    def resistor(self, ref, value, at, horizontal, dnp=False, text_at=None, footprint="", lcsc=None):
        x, y = at
        text_at = text_at or (((x, y - 3.81), (x, y + 3.81)) if horizontal
                              else ((x + 2.54, y - 1.27), (x + 2.54, y + 1.27)))
        return self.symbol("Device:R", ref, value, at, ["1", "2"], rot=90 if horizontal else 0,
                           ref_at=text_at[0], value_at=text_at[1], dnp=dnp, footprint=footprint, lcsc=lcsc)

    def write(self, path, paper, title_block, libs, items):
        body = "\n".join(items)
        path.write_text(f'''(kicad_sch
\t(version 20260306)
\t(generator "eeschema")
\t(generator_version "10.0")
\t(uuid "{self.uuid}")
\t(paper "{paper}")
\t{title_block}
\t(lib_symbols
{libs}\t)
{body}
\t(sheet_instances (path "/" (page "1")))
\t(embedded_fonts no)
)
''')


def title_block(title, date, revision, *comments):
    lines = " ".join(f'(comment {n} "{text}")' for n, text in enumerate(comments, 1))
    return f'(title_block (title "{title}") (date "{date}") (rev "{revision}") (company "Plokkke") {lines})'


def wire(points, color=""):
    stroke = f"(stroke (width 0.25) (type default) {color})" if color else "(stroke (width 0.25) (type default))"
    return [f'\t(wire (pts (xy {a[0]} {a[1]}) (xy {b[0]} {b[1]})) {stroke} (uuid "{u()}"))'
            for a, b in zip(points, points[1:])]


def text(value, at, size=1.27, bold=False, color=""):
    font = f"(font (size {size} {size}){' (bold yes)' if bold else ''}{' ' + color if color else ''})"
    return (f'\t(text "{value}" (exclude_from_sim no) (at {at[0]} {at[1]} 0) (effects {font} '
            f'(justify left bottom)) (uuid "{u()}"))')


def label(name, at, angle=0):
    justify = "right" if angle == 180 else "left"
    return (f'\t(label "{name}" (at {at[0]} {at[1]} {angle}) {FONT[:-1]} (justify {justify} bottom)) '
            f'(uuid "{u()}"))')


def no_connect(at):
    return f'\t(no_connect (at {at[0]} {at[1]}) (uuid "{u()}"))'


def junction(at):
    return f'\t(junction (at {at[0]} {at[1]}) (diameter 0) (color 0 0 0 0) (uuid "{u()}"))'


def rectangle(start, end, width, kind, color):
    return (f'\t(rectangle (start {start[0]} {start[1]}) (end {end[0]} {end[1]}) '
            f'(stroke (width {width}) (type {kind}) {color}) (fill (type none)) (uuid "{u()}"))')


def block(start, end, title):
    """A grey dashed block around one function, titled at its top left."""
    return [rectangle(start, end, 0.15, "dash", BLOCK_GREY), text(title, (start[0] + 1.27, start[1] + 3.81), bold=True)]


def project_files(root, name, module, description):
    """The project symbol library (the module), its table and a minimal project file."""
    (root / f"{name}.kicad_sym").write_text(
        f'(kicad_symbol_lib\n\t(version 20241209)\n\t(generator "{name}")\n{module.symbol(module.name)})\n')
    (root / "sym-lib-table").write_text(
        f'(sym_lib_table\n  (version 7)\n  (lib (name "{name}")(type "KiCad")(uri "${{KIPRJMOD}}/{name}.kicad_sym")'
        f'(options "")(descr "{description}"))\n)\n')
    (root / f"{name}.kicad_pro").write_text(f'{{\n  "meta": {{\n    "filename": "{name}.kicad_pro",\n    "version": 3\n  }}\n}}\n')
