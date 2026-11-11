#!/usr/bin/env python3
"""Talks to the MITM board over USB, on the bench only (never on the radiator: mains potential).

Usage: firmware/tools/console.py [--port /dev/cu.usbmodem1101] [--seconds 30] ["control setpoint" ...]
Sends each order (one line, named like the MQTT commands, firmware/mitm/src/console.rs), then prints the board's
logs for the given time. Opening the port does not restart the board, unlike `espflash monitor`.
"""
import argparse
import os
import select
import sys
import termios
import time
import tty


def options():
    parser = argparse.ArgumentParser()
    parser.add_argument("--port", default="/dev/cu.usbmodem1101")
    parser.add_argument("--seconds", type=float, default=30)
    parser.add_argument("orders", nargs="*")
    return parser.parse_args()


def follow(port, seconds):
    end = time.monotonic() + seconds
    while (left := end - time.monotonic()) > 0:
        if select.select([port], [], [], left)[0]:
            sys.stdout.write(os.read(port, 1024).decode(errors="replace"))
            sys.stdout.flush()


def main():
    args = options()
    port = os.open(args.port, os.O_RDWR | os.O_NOCTTY)
    try:
        tty.setraw(port)
        termios.tcflush(port, termios.TCIFLUSH)
        for order in args.orders:
            os.write(port, order.encode() + b"\n")
            time.sleep(0.2)
        follow(port, args.seconds)
    finally:
        os.close(port)


main()
