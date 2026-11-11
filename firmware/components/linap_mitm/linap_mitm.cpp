#include "linap_mitm.h"

#include <cmath>

#include "commander_tx.h"
#include "esphome/core/hal.h"
#include "esphome/core/helpers.h"
#include "esphome/core/log.h"
#include "power_side.h"

namespace esphome {
namespace linap_mitm {

static const char *const TAG = "linap_mitm";
static const char *const POWER_SIDE = "power board <-> MITM";
static const char *const COMMANDER_SIDE = "MITM <-> commander";
static constexpr uint32_t REPORT_MS = 10000;

std::optional<float> Counts::percent() const {
  if (this->polls == 0)
    return std::nullopt;
  return 100.0f * this->answered / this->polls;
}

void LinapMitm::setup() {
  this->pref_ = global_preferences->make_preference<Saved>(fnv1_hash("linap_mitm_state"), true);
  Saved saved{};
  if (this->pref_.load(&saved) && saved.mode <= static_cast<uint8_t>(linap::Mode::BOOST)) {
    this->mitm_.restore(static_cast<linap::Mode>(saved.mode), saved.halves);
    ESP_LOGI(TAG, "Saved state restored: %s, %.1f °C", this->mode().c_str(), this->setpoint());
  }
  for (uint8_t poll : {linap::header::STATE, linap::header::PERIODIC})
    power_side::set_reply(poll, *this->mitm_.reply_for(poll));
  power_side::start();
  commander_tx::start();
  this->set_interval("slot", linap::SLOT_MS, [this] { this->send_slot(); });
  this->set_interval("report", REPORT_MS, [this] { this->report(); });
}

void LinapMitm::loop() {
  this->read_power_side();
  if (this->mitm_.expire(millis())) {
    ESP_LOGI(TAG, "Ordered Boost over");
    this->changed();
  }
}

// The state changed: the power board gets it at its next poll, and it is saved for the next start-up
void LinapMitm::changed() {
  power_side::set_reply(linap::header::STATE, *this->mitm_.reply_for(linap::header::STATE));
  const Saved saved{static_cast<uint8_t>(this->mitm_.mode()), this->mitm_.halves()};
  this->pref_.save(&saved);
  ESP_LOGI(TAG, "State: %s, %.1f °C", this->mode().c_str(), this->setpoint());
}

void LinapMitm::order_mode(const std::string &name) {
  const auto mode = linap::parse_mode(name);
  if (!mode) {
    ESP_LOGW(TAG, "Unknown mode %s", name.c_str());
    return;
  }
  this->mitm_.set_mode(*mode, millis());
  this->changed();
}

void LinapMitm::order_setpoint(float celsius) {
  const auto halves = linap::setpoint_halves(celsius);
  if (!halves) {
    ESP_LOGW(TAG, "Setpoint %.1f °C out of range", celsius);
    return;
  }
  this->mitm_.set_setpoint(*halves);
  this->changed();
}

void LinapMitm::set_reply_gap_bits(float bits) {
  power_side::set_reply_gap_half_bits(static_cast<uint8_t>(std::lround(bits * 2)));
}

std::optional<std::string> LinapMitm::power_board() const {
  if (!this->power_state_)
    return std::nullopt;
  return std::string(linap::power_name(*this->power_state_));
}

std::optional<bool> LinapMitm::heating() const {
  if (!this->power_state_)
    return std::nullopt;
  return linap::is_heating(*this->power_state_);
}

void LinapMitm::send_slot() {
  const auto bytes = this->power_board_.next_slot(millis());
  if (bytes.empty())
    return;
  if (!commander_tx::send(bytes)) {
    ESP_LOGW(TAG, "Commander side: queue full");
    return;
  }
  if (linap::header::is_poll(bytes.back().byte))
    this->commander_counts_.polls++;
}

void LinapMitm::read_power_side() {
  HeardByte heard;
  while (power_side::next_heard(heard)) {
    this->power_counts_.bytes++;
    const linap::Heard meaning = this->power_wire_.push(heard.byte, heard.at_us);
    if (meaning.poll)
      this->power_counts_.polls++;
    if (meaning.frame)
      this->on_power_frame(*meaning.frame, heard.at_us);
  }
}

void LinapMitm::on_power_frame(const linap::Frame &frame, uint32_t) {
  if (linap::header::is_poll(frame.head()) && frame.is_valid())
    this->power_counts_.answered++;
  const auto state = linap::decode_power(frame);
  if (state && state != this->power_state_) {
    this->power_state_ = state;
    ESP_LOGI(TAG, "Power board: %s", linap::power_name(*state));
  }
  this->log_frame(POWER_SIDE, frame);
}

// One burst at a time: our own frame, or our poll byte with the commander's answer
void LinapMitm::on_commander_levels(const remote_base::RawTimings &timings) {
  const std::vector<uint8_t> bytes = linap::uart_bytes(timings);
  this->commander_counts_.bytes += bytes.size();
  for (size_t start = 0; start + 5 <= bytes.size(); start += 5) {
    linap::Frame frame;
    std::copy(bytes.begin() + start, bytes.begin() + start + 5, frame.bytes.begin());
    if (linap::header::is_poll(frame.head()) && frame.is_valid())
      this->commander_counts_.answered++;
    if (this->mitm_.from_commander(frame)) {
      ESP_LOGI(TAG, "Keys changed the state");
      this->changed();
    }
    this->log_frame(COMMANDER_SIDE, frame);
  }
}

// When the frame log is on: each frame that differs from the last one of its header, on its side
void LinapMitm::log_frame(const char *side, const linap::Frame &frame) {
  if (!this->frame_log_)
    return;
  auto &last = side == POWER_SIDE ? this->last_power_ : this->last_commander_;
  const auto previous = last.find(frame.head());
  if (previous != last.end() && previous->second == frame)
    return;
  last[frame.head()] = frame;
  ESP_LOGI(TAG, "%s: %s%s", side, linap::hex(frame).c_str(), frame.is_valid() ? "" : " (invalid)");
}

void LinapMitm::report() {
  this->power_report_ = this->power_counts_;
  this->commander_report_ = this->commander_counts_;
  this->power_counts_ = this->commander_counts_ = Counts{};
  const auto line = [](const Counts &counts) {
    return str_sprintf("%u bytes heard, %u polls, %u answered", counts.bytes, counts.polls, counts.answered);
  };
  ESP_LOGI(TAG, "%s: %s; %s: %s", POWER_SIDE, line(this->power_report_).c_str(), COMMANDER_SIDE,
           line(this->commander_report_).c_str());
  if (power_side::overflows() > 0)
    ESP_LOGW(TAG, "Power board side: %u bytes lost so far", power_side::overflows());
}

}  // namespace linap_mitm
}  // namespace esphome
