#pragma once

#include "linap-frames.h"

// Polling cycle of the real power board, one frame per slot: 75, C7, 61 (the command board answers 75 and C7
// with 46). Injected frames take the next slot.
class PowerBoard {
 public:
  linap::Frame c7_state = linap::known(0xC7, "normal");

  linap::Frame next() {
    if (linap_bus.has_pending())
      return linap_bus.take_pending();
    this->slot_ = (this->slot_ + 1) % 3;
    return this->slot_ == 0 ? POLL_75 : this->slot_ == 1 ? this->c7_state : POLL_61;
  }

 private:
  const linap::Frame POLL_75 = linap::parse("75 83 B5 02");
  const linap::Frame POLL_61 = linap::parse("61 1C 4C 80");
  int slot_ = -1;
};

static PowerBoard power_board;
