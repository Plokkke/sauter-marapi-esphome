#pragma once
// The radiator's state, owned by the MITM (knowledge/architecture.md): the commander's keys and Home Assistant
// change it, the last change wins; the power board receives it at each poll. Pure logic, the time in ms.
#include "linap.h"

namespace linap {

enum class Mode : uint8_t { OFF, ON, BOOST };

inline const char *mode_name(Mode mode) {
  static const char *const NAMES[] = {"off", "on", "boost"};
  return NAMES[static_cast<uint8_t>(mode)];
}

inline std::optional<Mode> parse_mode(const std::string &text) {
  for (Mode mode : {Mode::OFF, Mode::ON, Mode::BOOST})
    if (text == mode_name(mode))
      return mode;
  return std::nullopt;
}

// Until the saved state or the keys give one: off, at 15 °C
constexpr uint8_t DEFAULT_HALVES = 30;
// A Boost ordered by Home Assistant stops by itself after this long, unless it is ordered again
constexpr uint32_t DEFAULT_BOOST_MS = 30 * 60 * 1000;

class Mitm {
 public:
  Mode mode() const { return this->mode_; }
  // The setpoint of the On mode, kept while the radiator is off or in Boost
  uint8_t halves() const { return this->halves_; }

  CommandState state() const {
    switch (this->mode_) {
      case Mode::ON:
        return CommandState::on(this->halves_);
      case Mode::BOOST:
        return CommandState::boost();
      default:
        return CommandState::off();
    }
  }

  // The answer to a poll byte of the power board: the 4 bytes that complete its frame
  std::optional<std::array<uint8_t, 4>> reply_for(uint8_t poll) const {
    if (poll == header::STATE)
      return this->state().frame().body();
    if (poll == header::PERIODIC)
      return Frame::make(header::PERIODIC, 0x83, 0x80, 0x89).body();  // constant, meaning unknown
    return std::nullopt;
  }

  // The state saved before the restart, at start-up: it wins over the commander's first answer. A Boost does
  // not outlive a restart.
  void restore(Mode mode, uint8_t halves) {
    this->halves_ = is_setpoint(halves) ? halves : DEFAULT_HALVES;
    this->mode_ = mode == Mode::BOOST ? Mode::ON : mode;
    this->restored_ = true;
  }

  // An order from Home Assistant. Ordering a Boost again restarts its clock: that is how it is extended.
  void set_mode(Mode mode, uint32_t now_ms) {
    if (mode == Mode::BOOST)
      this->boost_ = Boost{this->boost_ ? this->boost_->back : this->mode_, now_ms};
    else
      this->boost_.reset();
    this->mode_ = mode;
  }

  // An order from Home Assistant: this setpoint, on
  void set_setpoint(uint8_t halves) { this->apply(CommandState::on(halves)); }

  uint32_t boost_ms() const { return this->boost_ms_; }
  void set_boost_ms(uint32_t boost_ms) { this->boost_ms_ = boost_ms; }

  // Ends a Boost ordered by Home Assistant once its time is over; true when that just happened
  bool expire(uint32_t now_ms) {
    if (!this->boost_ || now_ms - this->boost_->since_ms <= this->boost_ms_)
      return false;
    this->mode_ = this->boost_->back;
    this->boost_.reset();
    return true;
  }

  // A frame completed by the commander; true when the keys changed the state. Its first answer is the fallback
  // when nothing was saved; after that, only its changes are orders.
  bool from_commander(const Frame &frame) {
    if (frame.head() != header::STATE || !frame.is_valid() || (this->keys_ && *this->keys_ == frame))
      return false;
    const bool first = !this->keys_;
    this->keys_ = frame;
    const auto state = CommandState::decode(frame);
    if (!state || (first && this->restored_))
      return false;
    const CommandState before = this->state();
    this->apply(*state);
    return this->state() != before;
  }

 private:
  struct Boost {
    Mode back;  // the mode to go back to
    uint32_t since_ms;
  };

  // The commander ends its own Boost: the clock is only for a Boost ordered by Home Assistant
  void apply(const CommandState &state) {
    if (state.kind == CommandState::ON) {
      this->halves_ = state.halves;
      this->mode_ = Mode::ON;
    } else {
      this->mode_ = state.kind == CommandState::BOOST ? Mode::BOOST : Mode::OFF;
    }
    this->boost_.reset();
  }

  Mode mode_ = Mode::OFF;
  uint8_t halves_ = DEFAULT_HALVES;
  std::optional<Frame> keys_;  // the commander's last answer: only a change of it is an order
  std::optional<Boost> boost_;
  uint32_t boost_ms_ = DEFAULT_BOOST_MS;
  bool restored_ = false;
};

}  // namespace linap
