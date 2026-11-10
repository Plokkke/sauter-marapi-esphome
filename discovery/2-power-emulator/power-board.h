#pragma once

#include "linap-frames.h"

// Polling cycle of the real power board: 5 slots of ~61 ms (305 ms cycle measured on the radiator), 75, empty,
// C7, empty, 61. The command board answers 75 and C7 with 46 in the empty slots. Injected frames take the next
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

 private:
  const linap::Frame POLL_75 = linap::parse("75 83 B5 02");
  const linap::Frame POLL_61 = linap::parse("61 1C 4C 80");
  int slot_ = -1;
};

static PowerBoard power_board;
