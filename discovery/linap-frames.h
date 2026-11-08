#pragma once

// Linap frames (knowledge/linap.md): 9600 baud 8N1, 5 bytes = header, 3 data bytes, XOR of the 3 data bytes.
#include <algorithm>
#include <cmath>
#include <cstdio>
#include <cstdlib>
#include <deque>
#include <map>
#include <string>
#include <vector>

namespace linap {

using Frame = std::vector<uint8_t>;

static const size_t FRAME_SIZE = 5;
static const float BIT_US = 1e6f / 9600;
static const int IDLE_BITS = 12;

inline Frame with_checksum(Frame frame) {
  frame.push_back(frame[1] ^ frame[2] ^ frame[3]);
  return frame;
}

inline bool checksum_ok(const Frame &frame) {
  return frame.size() == FRAME_SIZE && (frame[1] ^ frame[2] ^ frame[3]) == frame[4];
}

// "C7 83 E0 98" (header + 3 data bytes, hex) -> frame with its checksum; empty when the text is not that
inline Frame parse(const std::string &text) {
  Frame frame;
  const char *cursor = text.c_str();
  char *end = nullptr;
  for (long value = strtol(cursor, &end, 16); end != cursor; value = strtol(cursor, &end, 16)) {
    if (value < 0 || value > 0xFF)
      return {};
    frame.push_back(value);
    cursor = end;
  }
  return frame.size() == FRAME_SIZE - 1 ? with_checksum(frame) : Frame{};
}

inline std::string hex(const Frame &frame) {
  std::string out;
  char byte[4];
  for (uint8_t value : frame) {
    snprintf(byte, sizeof byte, out.empty() ? "%02X" : " %02X", value);
    out += byte;
  }
  return out;
}

// Frames seen in the captures, by name: single source for the web page choices and the log descriptions
struct KnownFrame {
  const char *name;
  const char *data;  // header + 3 data bytes
};
static const KnownFrame KNOWN_FRAMES[] = {
    {"normal", "C7 80 E0 38"}, {"boost", "C7 83 E0 98"}, {"heating", "C7 01 E0 38"}, {"off", "C7 80 E0 40"},
    {"boost", "46 38 80 80"},  {"off", "46 80 80 80"},   {"periodic", "70 83 80 89"},
};

// Known frame of this header and name, e.g. known(0xC7, "boost"); empty when unknown
inline Frame known(uint8_t header, const std::string &name) {
  for (const KnownFrame &entry : KNOWN_FRAMES) {
    const Frame frame = parse(entry.data);
    if (frame[0] == header && name == entry.name)
      return frame;
  }
  return {};
}

// Hex dump plus what we know of it
inline std::string describe(const Frame &frame) {
  const std::string text = hex(frame);
  if (!checksum_ok(frame))
    return text + " (bad checksum)";
  for (const KnownFrame &entry : KNOWN_FRAMES)
    if (parse(entry.data) == frame)
      return text + ": " + entry.name;
  if (frame[0] != 0x46 || frame[1] != 0xB0)
    return text;
  char meaning[48];
  snprintf(meaning, sizeof meaning, ": setpoint %.1f, %s", (frame[2] & 0x7F) / 2.0f,
           frame[2] & 0x80 ? "no heat demand" : "heat demand");
  return text + meaning;
}

// remote_receiver timings (pin inverted: positive = low level, negative = high level, in µs) -> UART bytes
inline Frame uart_bytes(const std::vector<int32_t> &timings) {
  std::vector<bool> bits;
  for (int32_t duration : timings)
    bits.insert(bits.end(), std::max(1L, lroundf(std::abs(duration) / BIT_US)), duration < 0);
  bits.insert(bits.end(), IDLE_BITS, true);
  Frame bytes;
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

inline std::vector<Frame> split_frames(const Frame &bytes) {
  std::vector<Frame> frames;
  for (size_t i = 0; i < bytes.size(); i += FRAME_SIZE)
    frames.emplace_back(bytes.begin() + i, bytes.begin() + std::min(bytes.size(), i + FRAME_SIZE));
  return frames;
}

// Our side of the bus: frames waiting to be sent, our own frames heard back, frames from the other board
class Bus {
 public:
  void sent(const Frame &frame) { this->last_sent_ = frame; }
  void inject(const Frame &frame) { this->pending_.push_back(frame); }
  bool has_pending() const { return !this->pending_.empty(); }

  Frame take_pending() {
    Frame frame = this->pending_.front();
    this->pending_.pop_front();
    return frame;
  }

  // Calls handle(frame, news) for each frame of the other board; news = differs from the last one of its header
  template<typename Handler> void receive(const std::vector<int32_t> &timings, Handler handle) {
    for (const Frame &frame : split_frames(uart_bytes(timings))) {
      if (frame == this->last_sent_) {
        this->echoes_++;
        continue;
      }
      this->heard_++;
      Frame &last = this->last_by_header_[frame.empty() ? 0 : frame[0]];
      const bool news = last != frame;
      last = frame;
      handle(frame, news);
    }
  }

  // Since the last report; no echo means Q1 does not pull DATA low
  std::string report() {
    char text[80];
    snprintf(text, sizeof text, "%d of our frames heard back, %d from the other board", this->echoes_, this->heard_);
    this->echoes_ = this->heard_ = 0;
    return text;
  }

 private:
  Frame last_sent_;
  std::deque<Frame> pending_;
  std::map<uint8_t, Frame> last_by_header_;
  int echoes_ = 0;
  int heard_ = 0;
};

}  // namespace linap

static linap::Bus linap_bus;
