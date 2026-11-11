#ifdef USE_ESP8266
#include "commander_tx.h"

#include <Arduino.h>

namespace esphome {
namespace linap_mitm {
namespace commander_tx {

namespace {

constexpr uint8_t PIN = 12;  // D6
constexpr uint32_t PIN_MASK = 1 << PIN;
// timer1 at 80 MHz (TIM_DIV1), whatever the CPU clock
constexpr uint32_t BIT_TICKS = 80000000 / 9600;
// Nothing to send: look at the queue again this much later
constexpr uint32_t IDLE_TICKS = 80000;  // 1 ms
// Shortest wait the NMI can be armed for
constexpr uint32_t MIN_TICKS = 400;
constexpr size_t QUEUE_SIZE = 64;

// Single producer (the loop), single consumer (the NMI); only DRAM data and IRAM code in the NMI
linap::Paced queue[QUEUE_SIZE];
volatile size_t head = 0;
volatile size_t tail = 0;
// Bits of the byte being sent, start bit first (low on DATA = pin high), then the 8 bits, then the stop bit
uint16_t bits = 0;
uint8_t bits_left = 0;

inline void IRAM_ATTR pull_data_low(bool low) {
  if (low)
    GPOS = PIN_MASK;
  else
    GPOC = PIN_MASK;
}

// Each tick sets DATA for the interval that starts, and arms the timer for its end
void IRAM_ATTR tick() {
  uint32_t ticks;
  bool low = false;
  if (bits_left > 0) {
    low = !(bits & 1);
    bits >>= 1;
    bits_left--;
    ticks = BIT_TICKS;
  } else if (tail != head) {
    const linap::Paced next = queue[tail];
    tail = (tail + 1) % QUEUE_SIZE;
    bits = static_cast<uint16_t>(next.byte) << 1 | 1 << 9;
    bits_left = 10;
    ticks = next.idle_bits > 0 ? next.idle_bits * BIT_TICKS : MIN_TICKS;
  } else {
    ticks = IDLE_TICKS;
  }
  T1L = ticks;
  pull_data_low(low);
}

}  // namespace

void start() {
  pinMode(PIN, OUTPUT);
  digitalWrite(PIN, LOW);  // DATA released
  ETS_FRC_TIMER1_NMI_INTR_ATTACH(tick);
  timer1_enable(TIM_DIV1, TIM_EDGE, TIM_SINGLE);
  timer1_write(IDLE_TICKS);
}

bool send(const std::vector<linap::Paced> &bytes) {
  const size_t used = (head + QUEUE_SIZE - tail) % QUEUE_SIZE;
  if (used + bytes.size() >= QUEUE_SIZE)
    return false;
  for (const linap::Paced &byte : bytes) {
    queue[head] = byte;
    __asm__ __volatile__("" ::: "memory");
    head = (head + 1) % QUEUE_SIZE;
  }
  return true;
}

}  // namespace commander_tx
}  // namespace linap_mitm
}  // namespace esphome
#endif
