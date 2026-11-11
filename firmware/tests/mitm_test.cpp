#include "check.h"
#include "mitm.h"
#include "power_board.h"

using namespace linap;

static Frame on(uint8_t halves) { return CommandState::on(halves).frame(); }

TEST(answers_its_own_state_without_a_commander) {
  const Mitm mitm;
  CHECK(mitm.reply_for(header::STATE) == CommandState::off().frame().body());
  CHECK(mitm.reply_for(header::PERIODIC) == Frame::make(0x70, 0x83, 0x80, 0x89).body());
  CHECK(!mitm.reply_for(0x75));
}

TEST(takes_the_first_answer_of_the_keys_when_nothing_was_saved) {
  Mitm mitm;
  CHECK(mitm.from_commander(on(29)));
  CHECK(mitm.state() == CommandState::on(29));
  CHECK(!mitm.from_commander(on(29)));
  CHECK(mitm.from_commander(CommandState::off().frame()));
  CHECK(mitm.state() == CommandState::off());
}

TEST(the_saved_state_wins_over_the_first_answer_of_the_keys) {
  Mitm mitm;
  mitm.restore(Mode::ON, 40);
  CHECK(!mitm.from_commander(on(29)));
  CHECK(mitm.state() == CommandState::on(40));
  CHECK(mitm.from_commander(on(31)));
  CHECK(mitm.state() == CommandState::on(31));
}

TEST(a_saved_boost_restores_on) {
  Mitm mitm;
  mitm.restore(Mode::BOOST, 40);
  CHECK(mitm.state() == CommandState::on(40));
}

TEST(the_last_order_wins) {
  Mitm mitm;
  mitm.from_commander(on(29));
  mitm.set_mode(Mode::OFF, 0);
  CHECK(!mitm.from_commander(on(29)));
  CHECK(mitm.state() == CommandState::off());
  CHECK(mitm.from_commander(on(30)));
  CHECK(mitm.state() == CommandState::on(30));
}

TEST(keeps_the_setpoint_while_off) {
  Mitm mitm;
  mitm.set_setpoint(44);
  mitm.set_mode(Mode::OFF, 0);
  mitm.set_mode(Mode::ON, 0);
  CHECK(mitm.state() == CommandState::on(44));
}

TEST(stops_an_ordered_boost_unless_it_is_ordered_again) {
  Mitm mitm;
  mitm.set_boost_ms(1000);
  mitm.set_mode(Mode::OFF, 0);
  mitm.set_mode(Mode::BOOST, 0);
  CHECK(!mitm.expire(900));
  mitm.set_mode(Mode::BOOST, 900);
  CHECK(!mitm.expire(1800));
  CHECK(mitm.expire(1901));
  CHECK(mitm.state() == CommandState::off());
}

TEST(leaves_a_boost_from_the_keys_to_the_commander) {
  Mitm mitm;
  mitm.set_boost_ms(1000);
  mitm.set_mode(Mode::BOOST, 0);
  mitm.from_commander(CommandState::boost().frame());
  CHECK(!mitm.expire(5000));
  CHECK(mitm.state() == CommandState::boost());
}

TEST(ignores_corrupted_and_unknown_answers) {
  Mitm mitm;
  mitm.set_mode(Mode::BOOST, 0);
  CHECK(!mitm.from_commander(Frame{{0x46, 0xB0, 0x9D, 0x80, 0x00}}));
  CHECK(!mitm.from_commander(Frame::make(0x46, 0x01, 0x02, 0x04)));
  CHECK(mitm.state() == CommandState::boost());
}

TEST(parses_the_mode_names) {
  CHECK(parse_mode("boost") == Mode::BOOST);
  CHECK(!parse_mode("relay"));
}

static std::vector<uint8_t> heads(PowerBoard &board, int slots, uint32_t now_ms) {
  std::vector<uint8_t> found;
  for (int i = 0; i < slots; i++) {
    const auto bytes = board.next_slot(now_ms);
    found.push_back(bytes.empty() ? 0 : bytes[0].byte);
  }
  return found;
}

TEST(plays_the_five_slot_cycle) {
  PowerBoard board;
  CHECK(heads(board, 10, 0) == std::vector<uint8_t>({0x75, 0, 0xC7, 0, 0x61, 0x75, 0, 0xC7, 0, 0x61}));
}

TEST(paces_the_bytes_and_polls_after_75_and_c7) {
  PowerBoard board;
  const auto cycle = board.next_slot(0);
  CHECK(cycle.size() == 6);
  CHECK(cycle[1].idle_bits == AFTER_HEADER_BITS && cycle[2].idle_bits == BETWEEN_BYTES_BITS);
  CHECK(cycle[5].byte == 0x70 && cycle[5].idle_bits == 125);
  board.next_slot(0);
  CHECK(board.next_slot(300).back().byte == 0x46);
  board.next_slot(300);
  CHECK(board.next_slot(600).size() == 5);
}

TEST(polls_70_every_30_s) {
  PowerBoard board;
  CHECK(board.poll_after(PowerBoard::CYCLE, 0) == 0x70);
  CHECK(board.poll_after(PowerBoard::CYCLE, 300) == 0x46);
  CHECK(board.poll_after(PowerBoard::IDLE, 600) == 0x46);
  CHECK(!board.poll_after(PowerBoard::TAIL, 900));
  CHECK(board.poll_after(PowerBoard::CYCLE, 30000) == 0x70);
}

TEST(its_frames_are_valid) {
  CHECK(PowerBoard::CYCLE.is_valid() && PowerBoard::IDLE.is_valid() && PowerBoard::TAIL.is_valid());
}
