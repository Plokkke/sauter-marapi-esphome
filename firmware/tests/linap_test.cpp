#include "check.h"
#include "linap.h"

using namespace linap;

TEST(adds_the_parity_bit_only_when_needed) {
  CHECK(with_parity(0x46) == 0x46);
  CHECK(with_parity(0x1D) == 0x9D);
  CHECK(with_parity(0x7E) == 0xFE);
}

TEST(every_byte_with_parity_is_odd) {
  for (int data = 0; data <= 0x7F; data++)
    CHECK(has_odd_parity(with_parity(data)));
}

TEST(rounds_setpoints_to_the_half_degree_in_range) {
  CHECK(setpoint_halves(14.5f) == 29);
  CHECK(setpoint_halves(19.8f) == 40);
  CHECK(!setpoint_halves(4.5f));
  CHECK(!setpoint_halves(30.5f));
}

TEST(encodes_the_captured_frames) {
  CHECK(CommandState::on(29).frame() == Frame::make(0x46, 0xB0, 0x9D, 0x80));
  CHECK(CommandState::on(41).frame() == Frame::make(0x46, 0xB0, 0x29, 0x80));
  CHECK(CommandState::boost().frame() == Frame::make(0x46, 0x38, 0x80, 0x80));
}

TEST(decodes_what_it_encodes) {
  for (CommandState state : {CommandState::off(), CommandState::boost(), CommandState::on(10),
                             CommandState::on(34), CommandState::on(60)})
    CHECK(CommandState::decode(state.frame()) == state);
}

TEST(does_not_decode_other_frames) {
  CHECK(!CommandState::decode(Frame::make(0x70, 0x83, 0x80, 0x89)));
  CHECK(!CommandState::decode(Frame::make(0x46, 0x83, 0x80, 0x89)));
  CHECK(!CommandState::decode(Frame{{0x46, 0xB0, 0x9D, 0x80, 0x00}}));
}

TEST(decodes_the_power_board_state) {
  CHECK(decode_power(Frame::make(0xC7, 0x83, 0xE0, 0x98)) == PowerState::BOOST);
  CHECK(is_heating(*decode_power(Frame::make(0xC7, 0x01, 0xE0, 0x38))));
  CHECK(!decode_power(Frame::make(0x75, 0x83, 0xB5, 0x02)));
}

static std::vector<Heard> feed(Wire &wire, std::vector<uint8_t> bytes, uint32_t start_us) {
  std::vector<Heard> heard;
  for (size_t i = 0; i < bytes.size(); i++)
    heard.push_back(wire.push(bytes[i], start_us + i * 1250));
  return heard;
}

TEST(assembles_a_frame_from_one_burst) {
  Wire wire;
  const auto heard = feed(wire, {0xC7, 0x80, 0xE0, 0x38, 0x58}, 0);
  CHECK(heard[4].frame == Frame::make(0xC7, 0x80, 0xE0, 0x38));
  for (const Heard &h : heard)
    CHECK(!h.poll);
}

TEST(sees_the_poll_byte_after_a_silence_and_the_answer_completing_it) {
  Wire wire;
  feed(wire, {0x75, 0x83, 0xB5, 0x02, 0x36}, 0);
  const auto heard = feed(wire, {0x46, 0xB0, 0x9D, 0x80, 0xAD}, 18000);
  CHECK(heard[0].poll == 0x46);
  CHECK(heard[4].frame == Frame::make(0x46, 0xB0, 0x9D, 0x80));
}

TEST(a_46_inside_a_frame_is_not_a_poll) {
  Wire wire;
  for (const Heard &h : feed(wire, {0x75, 0x46, 0x46, 0x02, 0x02}, 0))
    CHECK(!h.poll);
}

TEST(an_unanswered_poll_is_dropped_at_the_next_burst) {
  Wire wire;
  feed(wire, {0x46}, 0);
  const auto heard = feed(wire, {0x61, 0x1C, 0x4C, 0x80, 0xD0}, 40000);
  CHECK(heard[4].frame && heard[4].frame->head() == 0x61);
}

TEST(ignores_bytes_past_the_fifth) {
  Wire wire;
  const auto heard = feed(wire, {0x75, 0x83, 0xB5, 0x02, 0x36, 0x99}, 0);
  CHECK(!heard[5].frame && !heard[5].poll);
}

// The levels of a byte as the receiver reports them: start bit low, LSB first, stop bit high
static std::vector<int32_t> levels(std::vector<uint8_t> bytes) {
  std::vector<int32_t> timings;
  auto level = [&](bool high) {
    const int32_t bit = high ? -int32_t(BIT_US) : int32_t(BIT_US);
    if (!timings.empty() && (timings.back() < 0) == high)
      timings.back() += bit;
    else
      timings.push_back(bit);
  };
  for (uint8_t byte : bytes) {
    level(false);
    for (int i = 0; i < 8; i++)
      level(byte >> i & 1);
    level(true);
  }
  return timings;
}

TEST(reads_bytes_from_edge_timings) {
  const std::vector<uint8_t> bytes{0x46, 0xB0, 0x9D, 0x80, 0xAD};
  CHECK(uart_bytes(levels(bytes)) == bytes);
}
