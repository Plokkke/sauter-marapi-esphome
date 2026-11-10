# sauter-marapi-esphome

[Français](README.md) · **English**

Control your Sauter Marapi towel radiator from Home Assistant while keeping its own screen and keys: a small Wi-Fi board sits on the radiator's internal bus.

![The Marapi control box, opened](knowledge/photos/command-board-in-box.jpg)

> [!NOTE]
> **Project status.** The internal bus protocol is decoded, and both halves of the design are proven with an ESP on the bench: it drives the power board on the radiator, and it reads the control box on the bench. The final board, its firmware and its enclosure are still to be done.

## What it does

- **Monitor the radiator**: setpoint, Boost and off as set on the keys, heating in progress.
- **Control it from Home Assistant**: setpoint, Boost (fan), off.
- **Keep the original appliance**: the screen and keys keep working as before; the last one to speak, key or Home Assistant, wins.
- **Fail-safe**: with the board off or broken, a relay joins the radiator's two boards directly, and it works as it did originally.

## Compatibility

Studied on a **Sauter Marapi tactile 1750 W** (750 W fluid element and 1000 W fan), control box `HM011 TD1N01 RAD`. Towel radiators and heaters of the Atlantic group (Atlantic, Thermor, Sauter) that show "Err 10" when their internal ribbon is unplugged probably use the same bus: check with the sniffer in `discovery/`.

## How it works

The radiator has two boards, joined by a three-wire ribbon (ground, 5 V, data):

- the **control box** (screen, capacitive keys) chooses the setpoint, the mode and the Boost;
- the **power board** heats the fluid and runs the fan, based on its own temperature sensor. It leads the exchange: it **polls** the control box several times a second, and the control box answers with its state.

The board cuts the data wire and sits between the two (*man in the middle*). By default it **relays** every poll and every answer, and Home Assistant sees what happens. On Home Assistant's request it **answers instead of** the control box, until the next key press.

```mermaid
flowchart LR
  HA[Home Assistant] <-- Wi-Fi --> C3[ESP32-C3<br>ESPHome]
  CMD[Control box<br>screen, keys] <-- data --> C3
  C3 <-- data --> PWR[Power board<br>fluid, fan]
  PWR -. 5 V and ground, direct .- CMD
  C3 --- R[Fail-safe relay:<br>joins the data wire<br>when the board stops]
```

**Limitation**: the control box accepts no order over the bus. Its screen always shows what was set on its keys: a Boost started by Home Assistant does not show there.

## The internal bus in a nutshell

The frame-by-frame details are in [`knowledge/linap.md`](knowledge/linap.md) (in French).

- One data wire, **9600 baud, 7 data bits and odd parity**. Frames are **5 bytes**: a header, 3 data bytes, then their XOR.
- The power board sends `75`, `C7` and `61` in a loop (305 ms cycle). After `75` and `C7` it sends the lone byte `46`: the poll. The control box answers **within the next bit** with 3 bytes and the checksum:

| Answer to `46` | Meaning |
|---|---|
| `B0 cc 80` | on, setpoint = `cc` without its bit 7, divided by 2 |
| `38 80 80` | Boost |
| `80 80 80` | off |

- The power board reports its state in `C7`: `80 E0 38` idle, `01 E0 38` heating, `83 E0 98` Boost, `80 E0 40` off.
- **The power board obeys whoever answers its polls in time.** **The control box cannot be driven**: it only answers the `46` and `70` polls, and none of the 280 or so frames tried changed its setpoint or its screen.

## Decoding the protocol

There is no documentation of this bus. It was decoded with a **Wemos D1 mini** wired to the ribbon, in three stages. The firmwares, the bench schematic and the tools are in [`discovery/`](discovery/README.md) (in French).

| Stage | The ESP… | Result |
|---|---|---|
| 1. Listen | listens to the ribbon between both boards | frame format, setpoint, Boost, off |
| 2. Fake power board | polls the control box alone, on the bench, off mains | it answers with everything done on its keys, but accepts no order |
| 3. Fake control box | answers the power board, on the radiator | it follows the setpoint, the Boost and off |

> [!WARNING]
> **The internal bus is at mains potential.** During the tests on the radiator, the ESP floats at that potential: lab supply with a floating output, Wi-Fi only, never USB nor an earthed device. Switch off the breaker before touching the setup. If in doubt, call an electrician.

## Hardware

> [!NOTE]
> Work in progress. The board drawn in `hardware/linap/` taps the bus in parallel. It has to be redrawn for the man in the middle, with two isolated channels, the fail-safe relay and a receiver fast enough to answer within the bit that follows the poll ([backlog](knowledge/backlog.md), in French).

What will not change:

- **ESP32-C3 super mini** running ESPHome, plugged into sockets.
- **Isolated supply** HLK-PM01 (230 V → 5 V), on the towel radiator's wall supply, with a **T1A fuse** and a **varistor**.
- **Isolated bus** through optocouplers: an H11L1M to read, an LTV-817 to pull the data line low, on each of the two channels.
- More than **6 mm** between the mains side (230 V and bus) and low voltage on the board.

KiCad 10 projects, shared library and fabrication export: [`hardware/`](hardware/README.md) (in French).

## Firmware

The shared ESPHome configuration (Wi-Fi, API, OTA) is in `firmware/`. The final board's firmware is still to be written; meanwhile the bench firmwares run on a D1 mini:

```sh
uv tool install esphome
cp firmware/secrets.example.yaml firmware/secrets.yaml   # Wi-Fi, API key, OTA password
esphome run discovery/1-sniffer/linap-sniffer.yaml       # first flash over USB, before any wiring
esphome logs discovery/1-sniffer/linap-sniffer.yaml | discovery/decode.py
```

## In Home Assistant

Planned entities:

| Entity | Use |
|---|---|
| Control box setpoint, Boost, off | what is set on the keys |
| Heating | is the radiator heating? |
| Imposed mode | relay (default), imposed setpoint, Boost, off |
| Imposed setpoint | setpoint sent to the power board in imposed mode |

The control box only sends its effective setpoint: in Eco, Home Assistant sees the Eco setpoint, without knowing it is Eco.

## Known limitations

- **Screen not synchronised**: what Home Assistant imposes does not show on the screen.
- **No temperature** yet: the power board has its sensor, but which frame carries the reading is not known yet.
- **Warranty**: opening the control box voids the warranty (2 years).

## Going further

- [`knowledge/linap.md`](knowledge/linap.md): the internal bus, frame by frame, and the Linap board.
- [`knowledge/architecture.md`](knowledge/architecture.md): the board's role, chosen design, rejected options, safety.
- [`knowledge/backlog.md`](knowledge/backlog.md): what is left to do.
- [`knowledge/fil-pilote.md`](knowledge/fil-pilote.md): first approach, through the pilot wire, set aside (board `hardware/pilot-wire/`).
- `knowledge/manuals/`: Sauter manual.

All of `knowledge/` is in French.

---

**Keywords**: Sauter Marapi tactile, electric towel radiator, towel warmer, bathroom fan heater, Groupe Atlantic, Thermor, Atlantic, internal bus, Linap link, Err 10, man in the middle, reverse engineering, ESPHome, Home Assistant, home automation, ESP32-C3, ESP32-C3 super mini, HLK-PM01, H11L1M, control board `HM011 TD1N01 RAD`, `74 60 5004`, STM32F091CCT6, KiCad PCB.
