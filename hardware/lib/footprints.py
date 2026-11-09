#!/usr/bin/env python3
"""Project footprints, drawn from the datasheets of the parts ordered at LCSC (see README, "Commander").

Writes every footprint into radiator-controller.pretty/, next to this script, or only the ones named as arguments.
"""
import os
import sys
import uuid
from pathlib import Path

LIBRARY = Path(__file__).parent / "radiator-controller.pretty"
MODELS = "${KICAD10_3DMODEL_DIR}"
KICAD_FOOTPRINTS = Path(os.environ.get("KICAD10_FOOTPRINT_DIR", "/Applications/KiCad/KiCad.app/Contents/SharedSupport/footprints"))
PITCH = 2.54
FONT = "\t\t(effects\n\t\t\t(font\n\t\t\t\t(size 1 1)\n\t\t\t\t(thickness 0.15)\n\t\t\t)\n\t\t)"

# ESP32-C3 super mini, measured: USB end at -y, components visible
C3_WIDTH, C3_LENGTH = 18.0, 22.0
C3_USB_OVERHANG, C3_USB_WIDTH = 1.5, 9.0
# 0.6 inch, breadboard compatible (ProtoSupplies spec); the castellated pads run past the holes to the edge
C3_ROW_SPACING = 6 * PITCH
C3_ANTENNA_KEEPOUT = 2.3

# hanxia HX PM2.54-1x8P TP H8.5-ZQ (LCSC C48641756), "A type": pin 1 leg on the left, legs alternate
SOCKET_PAD = (3.00, 1.02)
SOCKET_LEG_OFFSET = 1.5
SOCKET_BODY = (2.5, 20.82)

# Kefa KF128-5.08-2P-AA / -3P-AA (LCSC C474952 / C474953): 1.4 mm holes; body n x 5.08 wide, 10.7 deep, 14.1 high,
# pins 5.4 mm from the back
TERMINAL_PITCH, TERMINAL_DRILL = 5.08, 1.4
TERMINAL_BACK, TERMINAL_FRONT = 5.4, 5.3

# Bourns MOV-07D431K (LCSC C1528105): leads 0.6 +-0.1 mm at 5.0 +-0.8 mm; disc 9.0 max, 4.8 thick, 11 high
VARISTOR_PITCH, VARISTOR_DISC, VARISTOR_THICKNESS = 5.0, 9.0, 4.8

KF128_PARTS = {2: "LCSC C474952", 3: "LCSC C474953"}

# JST B2B/B4B-PH-K-S (LCSC C131337 / C131334): 0.5 mm square pins (0.71 mm diagonal), 0.9 mm holes as recommended by XUNPU
JST_PH_PINS = (2, 4)

# Break-off bridge, as KiKit draws them: 0.5 mm holes every 0.8 mm, set 0.25 mm into each board so the break stays flush
BRIDGE_WIDTH, BRIDGE_GAP = 3.0, 2.0
BITE_DRILL, BITE_PITCH, BITE_INSET = 0.5, 0.8, 0.25


def u():
    return str(uuid.uuid4())


def stroke(width):
    return f"\t\t(stroke\n\t\t\t(width {width})\n\t\t\t(type solid)\n\t\t)"


def rect(layer, x0, y0, x1, y1, width=0.1):
    return (f"\t(fp_rect\n\t\t(start {round(x0, 3)} {round(y0, 3)})\n\t\t(end {round(x1, 3)} {round(y1, 3)})\n"
            f'{stroke(width)}\n\t\t(fill no)\n\t\t(layer "{layer}")\n\t\t(uuid "{u()}")\n\t)')


def centred_rect(layer, half_x, half_y, width=0.1, dy=0.0):
    return rect(layer, -half_x, -half_y + dy, half_x, half_y + dy, width)


def text(kind, value, y, layer):
    head = f'\t(property "{kind}" "{value}"' if kind else f'\t(fp_text user "{value}"'
    return f'{head}\n\t\t(at 0 {round(y, 3)} 0)\n\t\t(layer "{layer}")\n\t\t(uuid "{u()}")\n{FONT}\n\t)'


def smd_pad(number, x, y, size_x, size_y):
    return (f'\t(pad "{number}" smd roundrect\n\t\t(at {round(x, 3)} {round(y, 3)})\n'
            f"\t\t(size {size_x} {size_y})\n"
            '\t\t(layers "F.Cu" "F.Mask" "F.Paste")\n\t\t(roundrect_rratio 0.15)\n'
            f'\t\t(uuid "{u()}")\n\t)')


def tht_pad(number, x, y, diameter, drill):
    shape = "rect" if number == "1" else "circle"
    return (f'\t(pad "{number}" thru_hole {shape}\n\t\t(at {round(x, 3)} {round(y, 3)})\n'
            f"\t\t(size {diameter} {diameter})\n\t\t(drill {drill})\n"
            '\t\t(layers "*.Cu" "*.Mask")\n'
            f'\t\t(uuid "{u()}")\n\t)')


def model(path, rotate_z=0, offset_y=0):
    return (f'\t(model "{MODELS}/{path}"\n\t\t(offset\n\t\t\t(xyz 0 {offset_y} 0)\n\t\t)\n'
            "\t\t(scale\n\t\t\t(xyz 1 1 1)\n\t\t)\n"
            f"\t\t(rotate\n\t\t\t(xyz 0 0 {rotate_z})\n\t\t)\n\t)")


def footprint(name, descr, attr, ref_y, items, models=()):
    parts = [
        f'(footprint "{name}"', "\t(version 20260206)", '\t(generator "radiator-controller")', '\t(layer "F.Cu")',
        f'\t(descr "{descr}")',
        text("Reference", "REF**", ref_y, "F.SilkS"), text("Value", name, -ref_y, "F.Fab"),
        f"\t(attr {attr})", *items, "\t(embedded_fonts no)", *models, ")",
    ]
    return "\n".join(parts) + "\n"


def body(x0, y0, x1, y1):
    """Fab outline of the part, silkscreen just outside it, courtyard 0.5 mm around."""
    return [rect("F.Fab", x0, y0, x1, y1), rect("F.SilkS", x0 - 0.12, y0 - 0.12, x1 + 0.12, y1 + 0.12, 0.12),
            rect("F.CrtYd", x0 - 0.5, y0 - 0.5, x1 + 0.5, y1 + 0.5, 0.05)]


def antenna_keepout():
    top = C3_LENGTH / 2 - C3_ANTENNA_KEEPOUT
    half = C3_ROW_SPACING / 2 - SOCKET_LEG_OFFSET - SOCKET_PAD[0] / 2 - 0.3
    xy = " ".join(f"(xy {round(x, 3)} {round(y, 3)})" for x, y in (
        (-half, top), (half, top), (half, C3_LENGTH / 2), (-half, C3_LENGTH / 2)))
    return ('\t(zone\n\t\t(net 0)\n\t\t(net_name "")\n\t\t(layers "F.Cu" "B.Cu")\n'
            f'\t\t(uuid "{u()}")\n\t\t(name "antenna_keepout")\n\t\t(hatch edge 0.5)\n'
            "\t\t(connect_pads\n\t\t\t(clearance 0)\n\t\t)\n\t\t(min_thickness 0.25)\n"
            "\t\t(filled_areas_thickness no)\n\t\t(keepout\n\t\t\t(tracks not_allowed)\n"
            "\t\t\t(vias not_allowed)\n\t\t\t(pads not_allowed)\n\t\t\t(copperpour not_allowed)\n"
            "\t\t\t(footprints allowed)\n\t\t)\n\t\t(placement\n\t\t\t(enabled no)\n"
            '\t\t\t(sheetname "")\n\t\t)\n\t\t(fill\n\t\t\t(thermal_gap 0.5)\n'
            f"\t\t\t(thermal_bridge_width 0.5)\n\t\t)\n\t\t(polygon\n\t\t\t(pts {xy})\n\t\t)\n\t)")


def c3_outline():
    """Board-only drawing of the plugged module: outline, USB overhang, pin 1 rows and antenna keepout."""
    half_w, half_l = C3_WIDTH / 2, C3_LENGTH / 2
    usb_top = -half_l - C3_USB_OVERHANG
    items = [
        rect("F.Fab", -half_w, -half_l, half_w, half_l),
        rect("F.Fab", -C3_USB_WIDTH / 2, usb_top, C3_USB_WIDTH / 2, -half_l),
        text(None, "USB", -half_l + 1.5, "F.Fab"), text(None, "ANTENNA", half_l - 1.0, "F.Fab"),
        text(None, "5V GND 3V3 IO4..0", 0, "F.Fab"), antenna_keepout(),
    ]
    descr = (f"ESP32-C3 super mini (18 x 22 mm, USB-C overhangs 1.5 mm) plugged into two 1x8 sockets "
             f"{C3_ROW_SPACING} mm apart; pins 1-8 (5V GND 3V3 GPIO4..0) at +x, 9-16 at -x, USB end first. "
             "Pins: first at 1.27 mm from the USB edge, 2.54 mm pitch")
    return footprint("ESP32-C3_SuperMini_Outline", descr, "board_only exclude_from_pos_files exclude_from_bom",
                     half_l + 1.5, items)


def socket():
    half_x = SOCKET_LEG_OFFSET + SOCKET_PAD[0] / 2
    pads = [smd_pad(str(n), -SOCKET_LEG_OFFSET if n % 2 else SOCKET_LEG_OFFSET, (n - 4.5) * PITCH, *SOCKET_PAD)
            for n in range(1, 9)]
    marker = (f"\t(fp_circle\n\t\t(center {-half_x - 0.6} {-3.5 * PITCH})\n\t\t(end {-half_x - 0.35} {-3.5 * PITCH})\n"
              f'{stroke(0.25)}\n\t\t(fill yes)\n\t\t(layer "F.SilkS")\n\t\t(uuid "{u()}")\n\t)')
    items = [centred_rect("F.Fab", SOCKET_BODY[0] / 2, SOCKET_BODY[1] / 2),
             centred_rect("F.SilkS", SOCKET_BODY[0] / 2 + 0.1, SOCKET_BODY[1] / 2 + 0.1, 0.12),
             centred_rect("F.CrtYd", half_x + 0.25, SOCKET_BODY[1] / 2 + 0.25, 0.05), marker, *pads]
    descr = ("1x8 female socket, 2.54 mm, SMD, staggered legs, 8.5 mm high: hanxia HX PM2.54-1x8P TP H8.5-ZQ "
             "(LCSC C48641756), land pattern from its datasheet (pads 1.02 x 3.00, centres 1.5 mm off the row)")
    return footprint("PinSocket_1x08_P2.54mm_SMD_HX-ZQ", descr, "smd", SOCKET_BODY[1] / 2 + 1.5, items,
                     [model("Connector_PinSocket_2.54mm.3dshapes/PinSocket_1x08_P2.54mm_Vertical_SMD_Pin1Left.step")])


def hlk_pm01():
    """Hi-Link datasheet: body 34 x 20; AC pins 2.3 mm in, at 12.5 / 7.5 mm; DC pins at 31.7 mm, 2.3 mm from edges."""
    origin = (2.3, 12.5)
    pins = {"1": (2.3, 12.5), "2": (2.3, 7.5), "3": (31.7, 17.7), "4": (31.7, 2.3)}
    pads = [tht_pad(n, x - origin[0], origin[1] - y, 2.2, 1.2) for n, (x, y) in pins.items()]
    x0, y0, x1, y1 = -origin[0], origin[1] - 20, 34 - origin[0], origin[1]
    items = [*body(x0, y0, x1, y1), *pads]
    descr = ("Hi-Link HLK-PM01 (LCSC C209903), pins from the Hi-Link datasheet (pitch tolerance +-8 %): "
             "AC 5.0 mm apart, DC 15.4 mm apart, 29.4 mm between the rows; 1.2 mm holes absorb the tolerance")
    return footprint("Converter_ACDC_HLK-PM01", descr, "through_hole", y1 + 1.5, items,
                     [model("Converter_ACDC.3dshapes/Converter_ACDC_Hi-Link_HLK-PMxx.step")])


def kf128(pins):
    """Wire entry at +y, like KiCad's Phoenix MKDS footprint it replaces, so the swap keeps the board orientation."""
    pads = [tht_pad(str(n + 1), n * TERMINAL_PITCH, 0, 2.5, TERMINAL_DRILL) for n in range(pins)]
    entry = text(None, "WIRES", TERMINAL_FRONT - 1.0, "F.Fab")
    items = [*body(-TERMINAL_PITCH / 2, -TERMINAL_BACK, (pins - 0.5) * TERMINAL_PITCH, TERMINAL_FRONT), entry, *pads]
    descr = (f"Kefa KF128-5.08-{pins}P-AA screw terminal ({KF128_PARTS[pins]}), from the Kefa drawing: 5.08 mm pitch, "
             f"1.4 mm holes, body {pins * TERMINAL_PITCH:.2f} x 10.7 mm, 14.1 mm high, pins 5.4 mm from the back, "
             "wire entry at +y; 250 V 24 A, up to 2.5 mm2")
    return footprint(f"TerminalBlock_Kefa_KF128-5.08_1x0{pins}", descr, "through_hole", TERMINAL_FRONT + 1.5, items)


def mov_07d():
    """Leads drawn inline: the 0.6 mm leads bend into 1.0 mm holes (KiCad's footprints stagger them by 2 mm)."""
    pads = [tht_pad(str(n + 1), n * VARISTOR_PITCH, 0, 1.8, 1.0) for n in range(2)]
    centre, half = VARISTOR_PITCH / 2, VARISTOR_DISC / 2
    items = [*body(centre - half, -VARISTOR_THICKNESS / 2, centre + half, VARISTOR_THICKNESS / 2), *pads]
    descr = ("Bourns MOV-07D431K disc varistor (LCSC C1528105), from the Bourns drawing: leads 0.6 mm at 5.0 +-0.8 mm, "
             "disc 9.0 mm max, 4.8 mm thick, 11 mm high")
    return footprint("RV_Disc_D9mm_W4.8mm_P5mm_Inline", descr, "through_hole", VARISTOR_THICKNESS / 2 + 1.5, items,
                     [model("Varistor.3dshapes/RV_Disc_D9mm_W4.8mm_P5mm.step", offset_y=1.15)])


def jst_ph(pins):
    """KiCad's own footprint, holes widened from 0.75 to 0.9 mm and pads from 1.2 to 1.4 mm to keep the ring."""
    name = f"JST_PH_B{pins}B-PH-K_1x0{pins}_P2.00mm_Vertical"
    source = (KICAD_FOOTPRINTS / "Connector_JST.pretty" / f"{name}.kicad_mod").read_text()
    return (source.replace(f'(footprint "{name}"', f'(footprint "{name}_Drill0.9"')
            .replace("(drill 0.75)", "(drill 0.9)")
            .replace("(size 1.2 1.75)", "(size 1.4 1.75)"))


def npth(x, y, drill):
    return (f'\t(pad "" np_thru_hole circle\n\t\t(at {round(x, 3)} {round(y, 3)})\n'
            f"\t\t(size {drill} {drill})\n\t\t(drill {drill})\n"
            '\t\t(layers "*.Cu" "*.Mask")\n'
            f'\t\t(uuid "{u()}")\n\t)')


def mouse_bites():
    """Bridge across the gap at y = 0; a row of holes on each board edge (y = -+GAP/2)."""
    count = int((BRIDGE_WIDTH - BITE_DRILL) // BITE_PITCH) + 1
    xs = [(n - (count - 1) / 2) * BITE_PITCH for n in range(count)]
    row = BRIDGE_GAP / 2 + BITE_INSET
    holes = [npth(x, y, BITE_DRILL) for y in (-row, row) for x in xs]
    items = [centred_rect("F.Fab", BRIDGE_WIDTH / 2, BRIDGE_GAP / 2), *holes]
    descr = (f"Break-off bridge {BRIDGE_WIDTH} mm wide across a {BRIDGE_GAP} mm gap: {count} NPTH holes of "
             f"{BITE_DRILL} mm every {BITE_PITCH} mm on each edge, {BITE_INSET} mm inside the board")
    return footprint("Breakaway_MouseBites_3mm", descr, "board_only exclude_from_pos_files exclude_from_bom",
                     row + 1.2, items)


FOOTPRINTS = {
    "ESP32-C3_SuperMini_Outline": c3_outline,
    "PinSocket_1x08_P2.54mm_SMD_HX-ZQ": socket,
    "Converter_ACDC_HLK-PM01": hlk_pm01,
    "RV_Disc_D9mm_W4.8mm_P5mm_Inline": mov_07d,
    **{f"TerminalBlock_Kefa_KF128-5.08_1x0{n}": lambda n=n: kf128(n) for n in KF128_PARTS},
    **{f"JST_PH_B{n}B-PH-K_1x0{n}_P2.00mm_Vertical_Drill0.9": lambda n=n: jst_ph(n) for n in JST_PH_PINS},
    "Breakaway_MouseBites_3mm": mouse_bites,
}

if __name__ == "__main__":
    for name in sys.argv[1:] or FOOTPRINTS:
        (LIBRARY / f"{name}.kicad_mod").write_text(FOOTPRINTS[name]())
