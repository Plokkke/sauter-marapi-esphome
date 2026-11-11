#pragma once
// Linap bus of the Sauter Marapi towel radiator (knowledge/linap.md): pure logic, no I/O, no framework; the time
// comes from the caller. Tested on the Mac (firmware/tests/).
#include <array>
#include <cmath>
#include <cstdint>
#include <cstdlib>
#include <optional>
#include <string>
#include <vector>

namespace linap {

// 9600 baud, 7 data bits and odd parity, read and written as 8N1
constexpr uint32_t BIT_US = 104;

constexpr bool has_odd_parity(uint8_t byte) { return __builtin_popcount(byte) % 2 == 1; }

// 7 data bits plus the odd parity bit as bit 7
constexpr uint8_t with_parity(uint8_t data) {
  data &= 0x7F;
  return has_odd_parity(data) ? data : data | 0x80;
}

// Header bytes, with their parity bit, as read on the wire
namespace header {
constexpr uint8_t STATE = 0x46;     // poll for the command board state, and header of its answer
constexpr uint8_t PERIODIC = 0x70;  // sent about every 30 s instead of STATE after a CYCLE
constexpr uint8_t POWER = 0xC7;     // power board state
constexpr uint8_t CYCLE = 0x75;
constexpr bool is_poll(uint8_t byte) { return byte == STATE || byte == PERIODIC; }
}  // namespace header

// Header, 3 data bytes, then the XOR of the 3 data bytes
struct Frame {
  std::array<uint8_t, 5> bytes{};

  static constexpr Frame make(uint8_t head, uint8_t a, uint8_t b, uint8_t c) {
    return Frame{{head, a, b, c, static_cast<uint8_t>(a ^ b ^ c)}};
  }
  constexpr uint8_t head() const { return bytes[0]; }
  constexpr bool data_is(uint8_t a, uint8_t b, uint8_t c) const {
    return bytes[1] == a && bytes[2] == b && bytes[3] == c;
  }
  // What follows the header: the answer to a poll byte
  std::array<uint8_t, 4> body() const { return {bytes[1], bytes[2], bytes[3], bytes[4]}; }
  // Checksum and parity of every byte
  bool is_valid() const {
    if ((bytes[1] ^ bytes[2] ^ bytes[3]) != bytes[4])
      return false;
    for (uint8_t byte : bytes)
      if (!has_odd_parity(byte))
        return false;
    return true;
  }
  bool operator==(const Frame &other) const { return bytes == other.bytes; }
  bool operator!=(const Frame &other) const { return bytes != other.bytes; }
};

inline std::string hex(const uint8_t *bytes, size_t len) {
  static const char DIGITS[] = "0123456789ABCDEF";
  std::string text;
  for (size_t i = 0; i < len; i++) {
    if (i > 0)
      text += ' ';
    text += DIGITS[bytes[i] >> 4];
    text += DIGITS[bytes[i] & 0x0F];
  }
  return text;
}
inline std::string hex(const Frame &frame) { return hex(frame.bytes.data(), frame.bytes.size()); }

// Setpoint in half degrees, in the 7 data bits of the command board state: the range of its keys
constexpr uint8_t MIN_HALVES = 10;
constexpr uint8_t MAX_HALVES = 60;
constexpr bool is_setpoint(int halves) { return halves >= MIN_HALVES && halves <= MAX_HALVES; }

// A temperature in °C to the nearest half degree, when in range
inline std::optional<uint8_t> setpoint_halves(float celsius) {
  const long halves = std::lround(celsius * 2);
  if (!is_setpoint(halves))
    return std::nullopt;
  return static_cast<uint8_t>(halves);
}

// What the command board answers to the 46 poll: what the user set on its keys
struct CommandState {
  enum Kind : uint8_t { OFF, BOOST, ON } kind;
  uint8_t halves;  // the setpoint, for ON

  static constexpr CommandState off() { return {OFF, 0}; }
  static constexpr CommandState boost() { return {BOOST, 0}; }
  static constexpr CommandState on(uint8_t halves) { return {ON, halves}; }
  bool operator==(const CommandState &other) const {
    return kind == other.kind && (kind != ON || halves == other.halves);
  }
  bool operator!=(const CommandState &other) const { return !(*this == other); }

  Frame frame() const {
    switch (kind) {
      case OFF:
        return Frame::make(header::STATE, 0x80, 0x80, 0x80);
      case BOOST:
        return Frame::make(header::STATE, 0x38, 0x80, 0x80);
      default:
        return Frame::make(header::STATE, 0xB0, with_parity(halves), 0x80);
    }
  }

  // None for a frame of another header or data never seen
  static std::optional<CommandState> decode(const Frame &frame) {
    if (frame.head() != header::STATE || !frame.is_valid())
      return std::nullopt;
    if (frame.data_is(0x80, 0x80, 0x80))
      return off();
    if (frame.data_is(0x38, 0x80, 0x80))
      return boost();
    const uint8_t halves = frame.bytes[2] & 0x7F;
    if (frame.bytes[1] == 0xB0 && frame.bytes[3] == 0x80 && is_setpoint(halves))
      return on(halves);
    return std::nullopt;
  }
};

// What the power board reports in C7
enum class PowerState : uint8_t { IDLE, HEATING, BOOST, BOOST_ENDING, OFF, UNKNOWN };

inline std::optional<PowerState> decode_power(const Frame &frame) {
  if (frame.head() != header::POWER || !frame.is_valid())
    return std::nullopt;
  if (frame.data_is(0x80, 0xE0, 0x38))
    return PowerState::IDLE;
  if (frame.data_is(0x01, 0xE0, 0x38))
    return PowerState::HEATING;
  if (frame.data_is(0x83, 0xE0, 0x98))
    return PowerState::BOOST;
  if (frame.data_is(0x83, 0xE0, 0x38))
    return PowerState::BOOST_ENDING;
  if (frame.data_is(0x80, 0xE0, 0x40))
    return PowerState::OFF;
  return PowerState::UNKNOWN;
}

// The resistance or the fan is on
inline bool is_heating(PowerState state) { return state == PowerState::HEATING || state == PowerState::BOOST; }

inline const char *power_name(PowerState state) {
  static const char *const NAMES[] = {"idle", "heating", "boost", "boost_ending", "off", "unknown"};
  return NAMES[static_cast<uint8_t>(state)];
}

// Silence that separates two bursts: the bytes of a frame are at most 7 idle bits apart (≈ 1.8 ms from one byte
// to the next), the poll byte comes ≈ 13 ms after the frame before it
constexpr uint32_t BURST_GAP_US = 3000;

// What one byte heard on a wire means
struct Heard {
  std::optional<uint8_t> poll;   // a lone 46 or 70 after a silence: the poll the other board must complete
  std::optional<Frame> frame;    // the fifth byte of a burst: the frame it completes, valid or not
};

// Splits everything heard on one side of DATA (both boards, our own bytes included) into frames
class Wire {
 public:
  Heard push(uint8_t byte, uint32_t now_us) {
    const bool starts_burst = !this->started_ || now_us - this->last_us_ > BURST_GAP_US;
    this->started_ = true;
    this->last_us_ = now_us;
    if (starts_burst)
      this->len_ = 0;
    Heard heard;
    if (this->len_ == this->frame_.bytes.size())
      return heard;
    this->frame_.bytes[this->len_++] = byte;
    if (starts_burst && header::is_poll(byte))
      heard.poll = byte;
    if (this->len_ == this->frame_.bytes.size())
      heard.frame = this->frame_;
    return heard;
  }

 private:
  bool started_ = false;
  uint32_t last_us_ = 0;
  Frame frame_;
  size_t len_ = 0;
};

// Levels of DATA from an edge-timing receiver, pin inverted (positive = low, negative = high, in µs), to the
// bytes they carry, read as a UART: idle high, start bit low, LSB first, stop bit high
inline std::vector<uint8_t> uart_bytes(const std::vector<int32_t> &timings) {
  std::vector<bool> bits;
  for (int32_t duration : timings) {
    const long count = std::lround(std::abs(duration) / static_cast<float>(BIT_US));
    bits.insert(bits.end(), count < 1 ? 1 : count, duration < 0);
  }
  bits.insert(bits.end(), 12, true);
  std::vector<uint8_t> bytes;
  for (size_t i = 0; i + 10 <= bits.size(); i++) {
    if (bits[i])
      continue;
    uint8_t value = 0;
    for (int bit = 0; bit < 8; bit++)
      value |= bits[i + 1 + bit] << bit;
    bytes.push_back(value);
    i += 9;
  }
  return bytes;
}

}  // namespace linap
