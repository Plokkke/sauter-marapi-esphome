#!/usr/bin/env python3
"""Decodes the Linap frames logged by discovery/1-sniffer/linap-sniffer.yaml (ESPHome remote_receiver, dump: raw).

Live:  esphome logs discovery/1-sniffer/linap-sniffer.yaml | tee capture.log | discovery/tools/decode.py
Later: discovery/tools/decode.py capture.log --changes --ignore 75

A frame is the list of DATA levels in µs: positive = low, negative = high (the receiver pin is inverted).
Bytes are read as a UART (Linap: 9600 baud, 7 data bits + odd parity, read here as 8N1): idle high, start bit low,
LSB first, then the optional parity bit and a stop bit high. --baud 0 fits the standard baud rate per frame instead.
Linap frames are 5 bytes, the last one being the XOR of bytes 2 to 4: frames failing it are flagged. A 46 or 70
frame is the power board's lone poll byte followed, one bit later, by the command board's 4 bytes. Action markers logged by the firmware's guided
capture ("action" tag) are printed in place, so the frames between two markers follow that action.
"""
import argparse
import re
import sys
from collections import Counter
from dataclasses import dataclass, field

STANDARD_BAUDS = (300, 600, 1200, 2400, 4800, 9600, 14400, 19200, 28800, 38400, 57600, 76800, 115200)
ANSI = re.compile(r"\x1b\[[0-9;]*m")
LOG_LINE = re.compile(r"^(?:\[(?P<time>[\d:.]+)\])?(?P<tags>.*?\]): (?P<message>.*)$")
HEAD = "Received Raw:"
# The firmware flushes a 256-byte line buffer as soon as a value no longer fits: a shorter line ends its frame
LAST_LINE_MAX = 240
IDLE_BITS = 12
LINAP_BAUD = 9600


@dataclass
class Frame:
    time: str
    durations: list[int] = field(default_factory=list)


@dataclass
class Marker:
    time: str
    text: str


@dataclass
class Decoded:
    baud: int
    data: list[int]
    errors: int


def read_events(lines):
    """Frames and action markers, in log order."""
    frame = None
    for line in lines:
        match = LOG_LINE.match(ANSI.sub("", line.rstrip("\n")))
        raw = match and "[remote.raw" in match["tags"]
        message = match["message"] if raw else ""
        if frame and (not raw or message.startswith(HEAD)):
            yield frame
            frame = None
        if match and "[action" in match["tags"]:
            yield Marker(match["time"] or "", match["message"])
        if not raw or not (frame or message.startswith(HEAD)):
            continue
        frame = frame or Frame(match["time"] or "")
        frame.durations += [int(v) for v in re.findall(r"-?\d+", message)]
        if len(message) < LAST_LINE_MAX:
            yield frame
            frame = None
    if frame:
        yield frame


def fit_error(durations, baud):
    bit = 1e6 / baud
    ratios = [abs(d) / bit for d in durations]
    if min(ratios) < 0.6:
        return float("inf")
    return sum(abs(r - round(r)) for r in ratios) / len(ratios)


def best_baud(durations):
    return min(STANDARD_BAUDS, key=lambda baud: fit_error(durations, baud))


def to_bits(durations, baud):
    bit = 1e6 / baud
    bits = []
    for d in durations:
        bits += [0 if d > 0 else 1] * max(1, round(abs(d) / bit))
    return bits + [1] * IDLE_BITS


def parity_ok(data_bits, parity_bit, parity):
    if parity == "none":
        return True
    ones = sum(data_bits) + parity_bit
    return ones % 2 == (0 if parity == "even" else 1)


def uart_bytes(bits, data_size, parity):
    extra = 0 if parity == "none" else 1
    data, errors, i = [], 0, 0
    while i + data_size + extra + 1 < len(bits):
        if bits[i] == 1:
            i += 1
            continue
        data_bits = bits[i + 1:i + 1 + data_size]
        parity_bit = bits[i + 1 + data_size] if extra else 0
        stop = bits[i + 1 + data_size + extra]
        data.append(sum(b << k for k, b in enumerate(data_bits)))
        errors += stop != 1 or not parity_ok(data_bits, parity_bit, parity)
        i += 2 + data_size + extra
    return data, errors


def linap_checksum_ok(data):
    return len(data) == 5 and data[1] ^ data[2] ^ data[3] == data[4]


def decode(frame, args):
    baud = args.baud or best_baud(frame.durations)
    data, errors = uart_bytes(to_bits(frame.durations, baud), args.data_bits, args.parity)
    return Decoded(baud, data, errors)


def as_hex(data):
    return " ".join(f"{b:02X}" for b in data)


def show(frame, decoded, args):
    status = f"  ({decoded.errors} framing/parity errors)" if decoded.errors else ""
    status += "" if linap_checksum_ok(decoded.data) else "  (bad Linap checksum)"
    print(f"[{frame.time}] {decoded.baud} bauds, {len(decoded.data)} bytes: {as_hex(decoded.data)}{status}", flush=True)
    if args.show_bits:
        print("    bits:", "".join(map(str, to_bits(frame.durations, decoded.baud))), flush=True)


def summary(seen):
    print(f"\n{sum(seen.values())} frames, {len(seen)} distinct; most frequent:")
    for frame, count in seen.most_common(15):
        print(f"{count:6} x  {frame}")


def parse_args():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("log", nargs="?", help="captured log file (default: standard input)")
    parser.add_argument("--baud", type=int, default=LINAP_BAUD, help="baud rate; 0 fits it per frame")
    parser.add_argument("--parity", choices=("none", "even", "odd"), default="none")
    parser.add_argument("--data-bits", type=int, default=8)
    parser.add_argument("--show-bits", action="store_true", help="also print each frame as bits")
    parser.add_argument("--changes", action="store_true",
                        help="only print a frame when it differs from the previous one with the same first byte")
    parser.add_argument("--ignore", nargs="*", default=[], help="first bytes (hex) whose frames are not printed")
    return parser.parse_args()


def wanted(decoded, last_by_header, args):
    if not decoded.data:
        return True
    header = f"{decoded.data[0]:02X}"
    if header in (h.upper() for h in args.ignore):
        return False
    changed = last_by_header.get(header) != decoded.data
    last_by_header[header] = decoded.data
    return changed or not args.changes


def main():
    args = parse_args()
    seen, last_by_header = Counter(), {}
    source = open(args.log, encoding="utf-8", errors="replace") if args.log else sys.stdin
    try:
        for event in read_events(source):
            if isinstance(event, Marker):
                print(f"\n===== [{event.time}] {event.text}", flush=True)
                continue
            decoded = decode(event, args)
            if wanted(decoded, last_by_header, args):
                show(event, decoded, args)
            seen[as_hex(decoded.data)] += 1
    except KeyboardInterrupt:
        pass
    summary(seen)


if __name__ == "__main__":
    main()
