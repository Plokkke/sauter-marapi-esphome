#pragma once
// The power board side of DATA, on UART0 moved to D7 (GPIO13, RX through R1/R2) and D8 (GPIO15, TX into Q1, which
// inverts it). Served from our own UART interrupt: the answer to a poll byte must start in the bit that follows
// it (knowledge/linap.md), whatever Wi-Fi and the main loop are doing. Nothing else may use UART0: the logger
// is off on the serial port.
#include <array>
#include <cstdint>

namespace esphome {
namespace linap_mitm {

struct HeardByte {
  uint8_t byte;
  uint32_t at_us;
};

namespace power_side {

void start();
// The answer to this poll byte (46 or 70), from now on
void set_reply(uint8_t poll, const std::array<uint8_t, 4> &body);
// Idle time between the end of the poll byte and our answer, in half bits: the real commander leaves 1 bit
void set_reply_gap_half_bits(uint8_t half_bits);
// The next byte heard (both boards' and our own), in order; false when there is none
bool next_heard(HeardByte &heard);
// Bytes lost because the main loop did not read them in time
uint32_t overflows();

}  // namespace power_side
}  // namespace linap_mitm
}  // namespace esphome
