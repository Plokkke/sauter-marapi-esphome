#!/usr/bin/env python3
"""Updates the MITM board over Wi-Fi: builds the firmware, serves its app image, and asks the board to fetch it.

Usage: firmware/tools/ota.py   (needs mosquitto_pub: brew install mosquitto)
The board connects to this Mac (the direction that works on its Wi-Fi; no port is open on the board), reads the
OTA password then the image, writes it to its free OTA slot and reboots on it (firmware/mitm/src/ota.rs).
Secrets come from firmware/secrets.env and are never printed. The first flash goes over USB
(cd firmware/mitm && cargo run --release): it installs the partition table with the OTA slots.
"""
import os
import socket
import subprocess
import sys
import tempfile
from pathlib import Path

FIRMWARE = Path(__file__).resolve().parent.parent
MITM = FIRMWARE / "mitm"
ELF = MITM / "target/riscv32imc-unknown-none-elf/release/mitm"
IMAGE = MITM / "target/ota-image.bin"
TOPIC = "sauter-marapi/ota/set"
WAIT_S = 120


def secrets():
    pairs = (line.partition("=") for line in (FIRMWARE / "secrets.env").read_text().splitlines())
    return {key.strip(): value.strip().strip('"') for key, sep, value in pairs if sep and not key.startswith("#")}


def build():
    subprocess.run(["cargo", "build", "--release"], cwd=MITM, check=True)
    subprocess.run(["espflash", "save-image", "--chip", "esp32c3", str(ELF), str(IMAGE)], check=True,
                   stdout=subprocess.DEVNULL)
    return IMAGE.read_bytes()


def local_address(broker):
    """The address of this Mac on the network the broker is on."""
    with socket.socket(socket.AF_INET, socket.SOCK_DGRAM) as probe:
        probe.connect((broker, 1883))
        return probe.getsockname()[0]


def ask_board(env, address):
    """Credentials through mosquitto_pub's config file, never on its command line (visible to every process)."""
    with tempfile.TemporaryDirectory() as config:
        options = Path(config) / "mosquitto_pub"
        options.write_text(f"-u {env['MQTT_USER']}\n-P {env['MQTT_PASSWORD']}\n")
        options.chmod(0o600)
        subprocess.run(["mosquitto_pub", "-h", env["MQTT_HOST"], "-t", TOPIC, "-m", address], check=True,
                       env={**os.environ, "XDG_CONFIG_HOME": config})


def serve(server, image, password):
    board, peer = server.accept()
    with board:
        print(f"board connected from {peer[0]}, sending {len(image)} bytes")
        board.settimeout(WAIT_S)
        board.sendall(password.encode() + b"\n" + image)
        board.shutdown(socket.SHUT_WR)
        return board.recv(16).decode().strip()


def main():
    env = secrets()
    image = build()
    with socket.create_server(("0.0.0.0", 0)) as server:
        server.settimeout(WAIT_S)
        address = f"{local_address(env['MQTT_HOST'])}:{server.getsockname()[1]}"
        ask_board(env, address)
        print(f"waiting for the board on {address} (up to {WAIT_S} s)")
        try:
            reply = serve(server, image, env["OTA_PASSWORD"])
        except TimeoutError:
            sys.exit("the board did not come: is it online on MQTT?")
    print(f"board: {reply or 'no reply'}")
    sys.exit(0 if reply == "OK" else 1)


main()
