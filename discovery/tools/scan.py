#!/usr/bin/env python3
"""Probes the command board through the web page of linap-power-emulator.yaml: one request per second.

  discovery/tools/scan.py addresses   poll bytes 00-7F (parity added) after C7: which addresses does the command board answer?
  discovery/tools/scan.py known       the power board's own headers, each data byte set to 00, to each single bit, then to 7F
  discovery/tools/scan.py unknown     every other header, with data 80 80 80

The answers are read in the logs (esphome logs ... | tee scan.log): each test is marked by a "note: test ..." line.
Use the IP address (--host): resolving the .local name takes seconds on macOS.
"""
import argparse
import time
import urllib.parse
import urllib.request

PROBE = "number/Probe address/set"
SEND = "text/Send a frame once (4 hex bytes, checksum added)/set"
NOTE = "text/Note/set"
NORMAL_POLL = 0x46
BASES = {0xC7: (0x80, 0xE0, 0x38), 0x75: (0x83, 0xB5, 0x02), 0x61: (0x1C, 0x4C, 0x80),
         0x67: (0x80, 0x80, 0x80), 0x73: (0x37, 0x45, 0x02), 0x76: (0x02, 0x80, 0x89)}
POLLS = (0x46, 0x70)
WALK = (0x00, 0x01, 0x02, 0x04, 0x08, 0x10, 0x20, 0x40, 0x7F)


def parity(value):
    value &= 0x7F
    return value if bin(value).count("1") % 2 else value | 0x80


def hex_frame(header, data):
    return " ".join(f"{byte:02X}" for byte in (header, *data))


def known_frames():
    for header, base in BASES.items():
        for position in range(3):
            for value in WALK:
                data = list(base)
                data[position] = parity(value)
                yield hex_frame(header, data)


def unknown_frames():
    used = set(BASES) | set(POLLS)
    for value in range(128):
        if parity(value) not in used:
            yield hex_frame(parity(value), (0x80, 0x80, 0x80))


class Page:
    def __init__(self, host):
        self.host = host

    def post(self, path, value):
        url = f"http://{self.host}/{urllib.parse.quote(path)}?value={urllib.parse.quote(str(value))}"
        try:
            urllib.request.urlopen(urllib.request.Request(url, method="POST"), timeout=3).read()
        except OSError as error:
            print(f"failed: {path} {value}: {error}", flush=True)

    def run(self, tests, period_s):
        for path, value in tests:
            self.post(NOTE, f"test {value}")
            self.post(path, value)
            time.sleep(period_s)
        self.post(NOTE, "scan done")


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("scan", choices=("addresses", "known", "unknown"))
    parser.add_argument("--host", required=True, help="IP address of linap-power-emulator")
    parser.add_argument("--period", type=float, default=1.2, help="seconds per test")
    args = parser.parse_args()
    page = Page(args.host)
    if args.scan == "addresses":
        page.run(((PROBE, value) for value in range(128)), args.period)
        page.post(PROBE, NORMAL_POLL)
    else:
        page.run(((SEND, frame) for frame in (known_frames() if args.scan == "known" else unknown_frames())), args.period)


if __name__ == "__main__":
    main()
