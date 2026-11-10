#pragma once

#include "linap-frames.h"

// Polling cycle of the real power board: 5 slots of ~61 ms (305 ms cycle measured on the radiator), 75, empty,
// C7, empty, 61. About 13 ms after 75 and C7 it polls the command board with a lone header byte, 46 (70 after
// a 75 about every 30 s), which the command board completes with its 4 bytes. Injected frames take the next
// slot of the power board; an empty frame means: send nothing.
class PowerBoard {
 public:
  linap::Frame c7_state = linap::known(0xC7, "normal");

  linap::Frame next() {
    this->slot_ = (this->slot_ + 1) % 5;
    if (this->slot_ % 2 == 1)
      return {};
    if (linap_bus.has_pending())
      return linap_bus.take_pending();
    return this->slot_ == 0 ? POLL_75 : this->slot_ == 2 ? this->c7_state : POLL_61;
  }

  uint8_t probe = 0x46;  // poll byte sent after C7 instead of 46, to find other addresses (after 75: always 46)

  // Poll byte to send after this frame, -1 for none
  int poll_after(const linap::Frame &frame, uint32_t now_ms) {
    if (!linap::polls_after(frame))
      return -1;
    if (frame[0] == 0xC7)
      return this->probe;
    if (frame[0] == 0x75 && now_ms - this->last_70_ms_ >= PERIODIC_MS) {
      this->last_70_ms_ = now_ms;
      return 0x70;
    }
    return 0x46;
  }

 private:
  static const uint32_t PERIODIC_MS = 30000;
  const linap::Frame POLL_75 = linap::parse("75 83 B5 02");
  const linap::Frame POLL_61 = linap::parse("61 1C 4C 80");
  int slot_ = -1;
  uint32_t last_70_ms_ = 0;
};

static PowerBoard power_board;
