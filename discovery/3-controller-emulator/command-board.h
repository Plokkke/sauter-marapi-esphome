#pragma once

#include "linap-frames.h"

// Replies of the real command board, from the guided capture: 46 (its state) after 75, C7 and 76 (76 closes the
// 73-67-76 burst), but 70 instead of 46 after a 75 about every 30 s. Silent after 61, 73 and 67.
class CommandBoard {
 public:
  std::string mode = "normal";  // normal, boost or off
  float setpoint = 14.5f;
  bool heat_demand = false;

  linap::Frame state() const {
    if (this->mode != "normal")
      return linap::known(0x46, this->mode);
    const uint8_t value = lroundf(this->setpoint * 2) | (this->heat_demand ? 0 : 0x80);
    return linap::with_checksum({0x46, 0xB0, value, 0x80});
  }

  // Frame to send after this power board frame; empty when the real command board stays silent
  linap::Frame reply_to(const linap::Frame &frame, uint32_t now_ms) {
    if (!linap::checksum_ok(frame) || (frame[0] != 0x75 && frame[0] != 0xC7 && frame[0] != 0x76))
      return {};
    if (linap_bus.has_pending())
      return linap_bus.take_pending();
    if (frame[0] == 0x75 && now_ms - this->last_periodic_ms_ >= PERIODIC_MS) {
      this->last_periodic_ms_ = now_ms;
      return linap::known(0x70, "periodic");
    }
    return this->state();
  }

 private:
  static const uint32_t PERIODIC_MS = 30000;
  uint32_t last_periodic_ms_ = 0;
};

static CommandBoard command_board;
