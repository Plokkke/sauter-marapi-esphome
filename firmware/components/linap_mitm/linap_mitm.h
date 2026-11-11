#pragma once
// The MITM on the Wemos D1 mini (knowledge/architecture.md): owns the radiator's state, plays the commander in front
// of the power board and the power board in front of the commander, each side on its own. Home Assistant reads and
// orders it through the template entities of firmware/sauter-marapi.yaml.
#include <map>
#include <optional>
#include <string>

#include "esphome/components/remote_base/remote_base.h"
#include "esphome/core/component.h"
#include "esphome/core/preferences.h"
#include "mitm.h"
#include "power_board.h"

namespace esphome {
namespace linap_mitm {

// Bytes heard, polls sent or heard on one side, and the answers heard whole, since the last report
struct Counts {
  uint32_t bytes = 0;
  uint32_t polls = 0;
  uint32_t answered = 0;
  std::optional<float> percent() const;
};

class LinapMitm : public Component {
 public:
  void setup() override;
  void loop() override;
  float get_setup_priority() const override { return setup_priority::HARDWARE; }

  // What remote_receiver hears on the commander side (its own frames included)
  void on_commander_levels(const remote_base::RawTimings &timings);

  // For the template entities
  std::string mode() const { return linap::mode_name(this->mitm_.mode()); }
  float setpoint() const { return this->mitm_.halves() / 2.0f; }
  void order_mode(const std::string &name);
  void order_setpoint(float celsius);
  void set_boost_minutes(float minutes) { this->mitm_.set_boost_ms(minutes * 60000); }
  void set_reply_gap_bits(float bits);
  void set_frame_log(bool on) { this->frame_log_ = on; }
  std::optional<std::string> power_board() const;
  std::optional<bool> heating() const;
  std::optional<float> power_board_answered() const { return this->power_report_.percent(); }
  std::optional<float> commander_answered() const { return this->commander_report_.percent(); }

 protected:
  struct Saved {
    uint8_t mode;
    uint8_t halves;
  };

  void send_slot();
  void read_power_side();
  void on_power_frame(const linap::Frame &frame, uint32_t at_us);
  void log_frame(const char *side, const linap::Frame &frame);
  void changed();
  void report();

  linap::Mitm mitm_;
  linap::PowerBoard power_board_;
  linap::Wire power_wire_;
  std::optional<linap::PowerState> power_state_;
  ESPPreferenceObject pref_;
  Counts power_counts_, commander_counts_, power_report_, commander_report_;
  bool frame_log_ = false;
  std::map<uint8_t, linap::Frame> last_power_, last_commander_;
};

}  // namespace linap_mitm
}  // namespace esphome
