#pragma once
// Transmits on the commander side of DATA, D6 (GPIO12, into Q2, which inverts it), as a 9600 baud 8N1 UART: the
// ESP8266's second UART can only transmit on GPIO2, which must stay high at boot and would not with Q2 on it.
// Every bit is timed by the timer1 NMI, which nothing delays (not even the power board side's interrupt), so the
// spacing of the bytes is exact. The commander's answers are read by remote_receiver.
#include <vector>

#include "power_board.h"

namespace esphome {
namespace linap_mitm {
namespace commander_tx {

void start();
// Queues these bytes behind the ones not sent yet; false when there is no room left
bool send(const std::vector<linap::Paced> &bytes);

}  // namespace commander_tx
}  // namespace linap_mitm
}  // namespace esphome
