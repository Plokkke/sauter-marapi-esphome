#ifdef USE_ESP8266
#include "power_side.h"

#include <Arduino.h>

#include "linap.h"

namespace esphome {
namespace linap_mitm {
namespace power_side {

namespace {

constexpr uint32_t BAUD_RATE = 9600;
constexpr uint8_t RX_PIN = 13;  // D7, with UART0 swapped
constexpr uint8_t TX_PIN = 15;  // D8, with UART0 swapped
constexpr size_t RING_SIZE = 256;
constexpr uint32_t CYCLES_PER_US = F_CPU / 1000000;

// Only DRAM data and IRAM code from the interrupt: the flash cache may be off when it fires.
// Double buffered answers, one pair per poll byte (46, 70): the loop writes the idle one, then flips.
volatile uint8_t replies[2][2][4];
volatile uint8_t active[2];
volatile uint8_t reply_gap_half_bits = 2;
volatile bool ready[2];

// Single producer (the interrupt), single consumer (the loop)
HeardByte ring[RING_SIZE];
volatile size_t head = 0;
volatile size_t tail = 0;
volatile uint32_t lost = 0;
uint32_t last_byte_us = 0;

inline uint8_t slot_of(uint8_t poll) { return poll == linap::header::STATE ? 0 : 1; }

// The byte was read in the middle of its stop bit: half a bit, then the gap
void IRAM_ATTR answer(uint8_t poll, uint32_t start_cycles) {
  const uint8_t slot = slot_of(poll);
  if (!ready[slot])
    return;
  const uint32_t wait = linap::BIT_US * (1 + reply_gap_half_bits) / 2 * CYCLES_PER_US;
  while (esp_get_cycle_count() - start_cycles < wait) {
  }
  const volatile uint8_t *body = replies[slot][active[slot]];
  for (int i = 0; i < 4; i++)
    USF(0) = body[i];
}

void IRAM_ATTR keep(uint8_t byte, uint32_t at_us) {
  const size_t next = (head + 1) % RING_SIZE;
  if (next == tail) {
    lost = lost + 1;
    return;
  }
  ring[head] = {byte, at_us};
  head = next;
}

void IRAM_ATTR on_uart(void *, void *) {
  const uint32_t start_cycles = esp_get_cycle_count();
  const uint32_t status = USIS(0);
  while ((USS(0) >> USRXC) & 0xFF) {
    const uint8_t byte = USF(0);
    const uint32_t now_us = micros();
    if (now_us - last_byte_us > linap::BURST_GAP_US && linap::header::is_poll(byte))
      answer(byte, start_cycles);
    last_byte_us = now_us;
    keep(byte, now_us);
  }
  USIC(0) = status;
}

}  // namespace

void start() {
  ETS_UART_INTR_DISABLE();
  USD(0) = ESP8266_CLOCK / BAUD_RATE;
  // 8N1, the parity being bit 7 of each byte; TX inverted for the NPN
  USC0(0) = (3 << UCBN) | (1 << UCSBN) | (1 << UCTXI);
  USC0(0) |= (1 << UCRXRST) | (1 << UCTXRST);
  USC0(0) &= ~((1 << UCRXRST) | (1 << UCTXRST));
  USC1(0) = 1 << UCFFT;  // an interrupt for every byte
  USIC(0) = 0xFFFF;
  USIE(0) = 1 << UIFF;
  // As HardwareSerial::swap(), without its RX pull-up: with it, the R1/R2 divider no longer gets D7 below ~1.8 V
  pinMode(1, INPUT);
  pinMode(3, INPUT);
  pinMode(TX_PIN, FUNCTION_4);
  pinMode(RX_PIN, FUNCTION_4);
  GPF(RX_PIN) &= ~(1 << GPFPU);
  IOSWAP |= 1 << IOSWAPU0;
  ETS_UART_INTR_ATTACH(on_uart, nullptr);
  ETS_UART_INTR_ENABLE();
}

void set_reply(uint8_t poll, const std::array<uint8_t, 4> &body) {
  const uint8_t slot = slot_of(poll);
  const uint8_t idle = active[slot] ^ 1;
  for (int i = 0; i < 4; i++)
    replies[slot][idle][i] = body[i];
  active[slot] = idle;
  ready[slot] = true;
}

void set_reply_gap_half_bits(uint8_t half_bits) { reply_gap_half_bits = half_bits; }

bool next_heard(HeardByte &heard) {
  if (tail == head)
    return false;
  heard = ring[tail];
  tail = (tail + 1) % RING_SIZE;
  return true;
}

uint32_t overflows() { return lost; }

}  // namespace power_side
}  // namespace linap_mitm
}  // namespace esphome
#endif
