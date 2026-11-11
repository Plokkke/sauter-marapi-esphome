"""The plug-in boards drawn as themselves, shared by discovery/bench/ and hardware/ generators."""
from kicad_sheet import Module

# Pins in their physical place, top view, antenna up, USB down
D1_MINI = Module(
    name="WEMOS_D1_mini", reference="U", footprint="RF_Module:WEMOS_D1_mini_light",
    description="Wemos D1 mini (ESP8266), pins in their physical place, top view, USB down",
    datasheet="https://www.wemos.cc/en/latest/d1/d1_mini.html",
    left=(("RST", "1", "input"), ("A0", "2", "input"), ("D0/GPIO16", "3", "bidirectional"),
          ("D5/GPIO14", "4", "bidirectional"), ("D6/GPIO12", "5", "bidirectional"),
          ("D7/GPIO13", "6", "bidirectional"), ("D8/GPIO15", "7", "bidirectional"), ("3V3", "8", "power_out")),
    right=(("TX/GPIO1", "16", "bidirectional"), ("RX/GPIO3", "15", "bidirectional"),
           ("D1/GPIO5", "14", "bidirectional"), ("D2/GPIO4", "13", "bidirectional"),
           ("D3/GPIO0", "12", "bidirectional"), ("D4/GPIO2", "11", "bidirectional"), ("GND", "10", "power_in"),
           ("5V", "9", "power_in")),
    pin_x=15.24, top_y=8.89, body=(12.7, 13.97, -13.97), top_text="ANTENNA", bottom_text="USB")
