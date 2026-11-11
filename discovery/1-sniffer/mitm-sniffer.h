#pragma once
// One side of the MITM board's DATA (power board <-> MITM, or MITM <-> commander), read after the board's own divider: frames decoded, the ones that changed
// logged, polls and whole answers counted. linap-frames.h is included first.
namespace linap {

class Half {
 public:
  explicit Half(const char *tag) : tag_(tag) {}

  void receive(const std::vector<int32_t> &timings) {
    for (const Frame &frame : split_frames(uart_bytes(timings))) {
      this->frames_++;
      const bool poll = frame[0] == 0x46 || frame[0] == 0x70;
      const bool whole = frame.size() == FRAME_SIZE && checksum_ok(frame);
      this->polls_ += poll;
      this->answers_ += poll && whole;
      // Each poll left without a whole answer, not only the first: that is what goes wrong
      if (poll && !whole)
        ESP_LOGW(this->tag_, "unanswered: %s", describe(frame).c_str());
      Frame &last = this->last_by_header_[frame[0]];
      if (last != frame) {
        ESP_LOGI(this->tag_, "%s", describe(frame).c_str());
        last = frame;
      }
    }
  }

  // Since the last report
  std::string report() {
    char text[80];
    snprintf(text, sizeof text, "%d frames, %d polls, %d answered whole", this->frames_, this->polls_,
             this->answers_);
    this->frames_ = this->polls_ = this->answers_ = 0;
    return text;
  }

 private:
  const char *tag_;
  std::map<uint8_t, Frame> last_by_header_;
  int frames_ = 0;
  int polls_ = 0;
  int answers_ = 0;
};

}  // namespace linap

static linap::Half power_half("power board");
static linap::Half command_half("commander");
