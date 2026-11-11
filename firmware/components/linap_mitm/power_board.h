#pragma once
// The power board, played in front of the commander on the MITM's own clock (discovery/2-power-emulator): 75,
// nothing, C7, nothing, 61, and a poll byte after 75 and C7. The commander ignores what these frames say
// (knowledge/linap.md): they are constant, whatever the real power board does. Pure logic, the time in ms.
#include "linap.h"

namespace linap {

// One slot of the power board cycle: 5 slots make the ≈ 305 ms cycle measured on the radiator
constexpr uint32_t SLOT_MS = 61;
// From the end of a 75 or C7 frame to the poll byte, as measured on the radiator
constexpr uint32_t POLL_DELAY_MS = 13;
constexpr uint32_t PERIODIC_MS = 30000;
// Idle bits the power board leaves after the header, then between the other bytes (capture of 07/11/2026)
constexpr uint8_t AFTER_HEADER_BITS = 7;
constexpr uint8_t BETWEEN_BYTES_BITS = 2;

// One byte to send, after this many idle bits
struct Paced {
  uint8_t byte;
  uint16_t idle_bits;
};

class PowerBoard {
 public:
  static constexpr Frame CYCLE = Frame::make(header::CYCLE, 0x83, 0xB5, 0x02);
  static constexpr Frame IDLE = Frame::make(header::POWER, 0x80, 0xE0, 0x38);
  static constexpr Frame TAIL = Frame::make(0x61, 0x1C, 0x4C, 0x80);

  // The bytes of the next slot, spaced like the real power board's, its poll byte included
  std::vector<Paced> next_slot(uint32_t now_ms) {
    std::vector<Paced> bytes;
    const std::optional<Frame> frame = this->frame_of(this->slot_);
    this->slot_ = (this->slot_ + 1) % 5;
    if (!frame)
      return bytes;
    for (size_t i = 0; i < frame->bytes.size(); i++)
      bytes.push_back({frame->bytes[i], static_cast<uint16_t>(i == 0 ? 0 : i == 1 ? AFTER_HEADER_BITS
                                                                                  : BETWEEN_BYTES_BITS)});
    if (const auto poll = this->poll_after(*frame, now_ms))
      bytes.push_back({*poll, static_cast<uint16_t>(POLL_DELAY_MS * 1000 / BIT_US)});
    return bytes;
  }

  // The poll byte to send after this frame: 70 after a 75 about every 30 s, 46 otherwise
  std::optional<uint8_t> poll_after(const Frame &frame, uint32_t now_ms) {
    if (frame.head() == header::CYCLE && (!this->periodic_sent_ || now_ms - this->last_periodic_ms_ >= PERIODIC_MS)) {
      this->periodic_sent_ = true;
      this->last_periodic_ms_ = now_ms;
      return header::PERIODIC;
    }
    if (frame.head() == header::CYCLE || frame.head() == header::POWER)
      return header::STATE;
    return std::nullopt;
  }

 private:
  static std::optional<Frame> frame_of(uint8_t slot) {
    switch (slot) {
      case 0:
        return CYCLE;
      case 2:
        return IDLE;
      case 4:
        return TAIL;
      default:
        return std::nullopt;
    }
  }

  uint8_t slot_ = 0;
  bool periodic_sent_ = false;
  uint32_t last_periodic_ms_ = 0;
};

}  // namespace linap
