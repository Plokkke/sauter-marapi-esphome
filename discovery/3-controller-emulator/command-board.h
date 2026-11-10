#pragma once

#include "linap-frames.h"

// The real command board only speaks when polled: after the lone header byte 46 it sends the rest of its state
// frame, after 70 the rest of 70 83 80 89 (about every 30 s; meaning unknown).
class CommandBoard {
 public:
  std::string mode = "normal";  // normal, boost or off
  float setpoint = 14.5f;

  linap::Frame state() const {
    if (this->mode != "normal")
      return linap::known(0x46, this->mode);
    return linap::with_checksum({0x46, 0xB0, linap::with_parity(lroundf(this->setpoint * 2)), 0x80});
  }

  // Full frame for this poll byte (the power board sends its header, we send the rest); empty if not a poll
  linap::Frame frame_for(int poll) const {
    if (poll != 0x46 && poll != 0x70)
      return {};
    if (linap_bus.has_pending())
      return linap_bus.take_pending();
    return poll == 0x70 ? linap::known(0x70, "periodic") : this->state();
  }
};

static CommandBoard command_board;
