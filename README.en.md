# sauter-marapi-esphome

[Français](README.md) · **English**

DIY module that drives the **Sauter Marapi tactile 1750 W** electric towel radiator (fan-assisted bathroom heater, anthracite) from Home Assistant, through its French **6-order pilot wire** (*fil pilote*: Comfort, Comfort −1, Comfort −2, Eco, Frost protection, Off).

ESPHome firmware on a Wemos D1 mini (ESP8266), two MOC3043 optotriacs + diodes, a fail-safe relay that falls back to Eco, powered from the radiator's own 230 V wall supply.

> [!WARNING]
> **Study phase.** The add-on module (schematic, PCB, firmware) has not been built or tested yet: everything about it is **theoretical**. Only the manufacturer data (manual, pilot wire signals) and the photos of the original control board are facts; the reverse engineering leads (Boost key, internal link) are hypotheses. Check everything before building it.

**Keywords**: Sauter Marapi tactile, electric towel radiator, towel warmer, bathroom fan heater, fil pilote, pilot wire, Groupe Atlantic, ESPHome, Home Assistant, home automation, ESP8266, MOC3043, control board `HM011 TD1N01 RAD`, `74 60 5004`, STM32F091CCT6, capacitive touch keys, Boost button, Linap link, Err 10.

## Repository layout

| Folder | Content |
|---|---|
| `firmware/` | ESPHome configuration: `common.yaml` (Wi-Fi, API, OTA, LED, diagnostics), `controlleur-seche-serviettes.yaml` (pilot wire), `secrets.example.yaml` |
| `hardware/` | KiCad 10: schematic (`.kicad_sch`, `.svg` render), PCB (`.kicad_pcb`, `-top.png` / `-bottom.png` renders), 3D model `radiator-controller.step` for enclosure design, D1 mini symbol and footprint, isolation rules (`.kicad_dru`) |
| `enclosure/` | enclosure (to come) |
| `knowledge/` | design notes, in French: `architecture.md` (roles, contract, safety), `fil-pilote.md` (signals, schematic, bill of materials), `backlog.md` (including control board reverse engineering), Sauter manual (`manuals/`) |

## ⚠️ Mains voltage

The module works on 230 V mains, in a bathroom. Read the *Sécurité* section of `knowledge/fil-pilote.md` before wiring anything. Switch off the breaker before any work.

## Pilot wire

The order is the voltage between the pilot wire and neutral:

| Order | Signal |
|---|---|
| Comfort | nothing (0 V) |
| Eco | full 230 V sine wave |
| Frost protection | negative half-wave only |
| Off / load shedding | positive half-wave only |
| Comfort −1 / −2 | 230 V for 3 s / 7 s every 300 s |

On the radiator, select the **Programme Inactif** mode so that it obeys the pilot wire. The Boost (fan) cannot be triggered by the pilot wire: see `knowledge/backlog.md`.

## Hardware and wiring

![Schematic](hardware/radiator-controller.svg)

Open `hardware/radiator-controller.kicad_pro` in KiCad, or online with [KiCanvas](https://kicanvas.org/?github=https://github.com/Plokkke/sauter-marapi-esphome/blob/main/hardware/radiator-controller.kicad_sch). In short:

| D1 mini | Connected to |
|---|---|
| `5V` / `GND` | HLK-PM01 output |
| `D1` (GPIO5) | 220 Ω → MOC3043 U1 (positive half-wave) |
| `D2` (GPIO4) | 220 Ω → MOC3043 U2 (negative half-wave) |
| `D5` (GPIO14) | 1 kΩ → MMBT2222A → relay K1 (normally closed contact = Eco) |

## PCB

76 × 66 mm, 2 layers. Low voltage on the left, 230 V on the right, separated by a 1 mm slot; the HLK-PM01, MOC3043 and relay straddle the slot.

| Top | Bottom |
|---|---|
| ![Top](hardware/radiator-controller-top.png) | ![Bottom](hardware/radiator-controller-bottom.png) |

- **SRD relay**: its common pin (230 V) sits between the two coil pins (5 V), 3.5 mm away. A U-shaped slot surrounds it, open only towards the 230 V side, which brings the creepage distance above 6 mm.
- **Design rules** (`radiator-controller.kicad_dru`): 3 mm clearance and 6 mm creepage between 230 V and low voltage; 2.5 mm between 230 V conductors. DRC (`kicad-cli pcb drc --schematic-parity`) passes with no error; 18 intended warnings remain for silkscreen cut by the slot.
- **D1 mini** plugged into SMD socket strips, in physical pin order: `RST`/`TX` at the top, `3V3`/`5V` at the bottom. Terminal block labels are on the back of the board.

## Firmware

```sh
uv tool install esphome
cp firmware/secrets.example.yaml firmware/secrets.yaml   # then fill in the values
cd firmware
esphome run controlleur-seche-serviettes.yaml --device /dev/cu.usbserial-XXXX   # first flash, off mains
esphome run controlleur-seche-serviettes.yaml --device controlleur-seche-serviettes.local   # then over the air
esphome logs controlleur-seche-serviettes.yaml --device controlleur-seche-serviettes.local
```

## Home Assistant entity

`select.controlleur_seche_serviettes_pilot_wire_order`: `Comfort`, `Comfort -1`, `Comfort -2`, `Eco`, `Frost protection`, `Off`. On failure, or after 30 min without Home Assistant: `Eco`.
