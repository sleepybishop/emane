#include "transponderconfiguration.h"

extern "C" {
  void* rust_bentpipe_tc_new(uint16_t idx);
  void* rust_bentpipe_tc_clone(const void* ptr);
  void rust_bentpipe_tc_free(void* ptr);
  
  void rust_bentpipe_tc_set_rx_freq(void* ptr, uint64_t v);
  void rust_bentpipe_tc_set_rx_bw(void* ptr, uint64_t v);
  void rust_bentpipe_tc_set_rx_ant(void* ptr, uint16_t v);
  void rust_bentpipe_tc_set_curve(void* ptr, uint16_t v);
  void rust_bentpipe_tc_set_rx_action(void* ptr, uint16_t v);
  void rust_bentpipe_tc_set_rx_en(void* ptr, bool v);
  
  void rust_bentpipe_tc_set_tx_freq(void* ptr, uint64_t v);
  void rust_bentpipe_tc_set_tx_bw(void* ptr, uint64_t v);
  void rust_bentpipe_tc_set_tx_rate(void* ptr, uint64_t v);
  void rust_bentpipe_tc_set_tx_mtu(void* ptr, uint64_t v);
  void rust_bentpipe_tc_set_tx_ant(void* ptr, uint16_t v);
  void rust_bentpipe_tc_set_tx_pwr(void* ptr, double v);
  void rust_bentpipe_tc_set_tx_delay(void* ptr, uint64_t v);
  void rust_bentpipe_tc_set_tx_slots_per_frame(void* ptr, uint16_t v);
  void rust_bentpipe_tc_set_tx_slot_size(void* ptr, uint64_t v);
  void rust_bentpipe_tc_set_tx_en(void* ptr, bool v);
  
  uint16_t rust_bentpipe_tc_get_index(const void* ptr);
  uint64_t rust_bentpipe_tc_get_rx_freq(const void* ptr);
  uint64_t rust_bentpipe_tc_get_rx_bw(const void* ptr);
  uint16_t rust_bentpipe_tc_get_rx_ant(const void* ptr);
  uint16_t rust_bentpipe_tc_get_curve(const void* ptr);
  uint16_t rust_bentpipe_tc_get_rx_action(const void* ptr);
  bool rust_bentpipe_tc_get_rx_en(const void* ptr);
  
  uint64_t rust_bentpipe_tc_get_tx_freq(const void* ptr);
  uint64_t rust_bentpipe_tc_get_tx_bw(const void* ptr);
  uint64_t rust_bentpipe_tc_get_tx_rate(const void* ptr);
  uint64_t rust_bentpipe_tc_get_tx_mtu(const void* ptr);
  uint16_t rust_bentpipe_tc_get_tx_ant(const void* ptr);
  double rust_bentpipe_tc_get_tx_pwr(const void* ptr);
  uint64_t rust_bentpipe_tc_get_tx_delay(const void* ptr);
  uint16_t rust_bentpipe_tc_get_tx_slots_per_frame(const void* ptr);
  uint64_t rust_bentpipe_tc_get_tx_slot_size(const void* ptr);
  bool rust_bentpipe_tc_get_tx_en(const void* ptr);
}

EMANE::Models::BentPipe::TransponderConfiguration::TransponderConfiguration(TransponderIndex transponderIndex):
  transmitProcessTOS_{},
  transmitSlots_{},
  rust_obj_{rust_bentpipe_tc_new(transponderIndex)} {}

EMANE::Models::BentPipe::TransponderConfiguration::TransponderConfiguration(const TransponderConfiguration& other):
  transmitProcessTOS_(other.transmitProcessTOS_),
  transmitSlots_(other.transmitSlots_),
  rust_obj_(rust_bentpipe_tc_clone(const_cast<void*>(other.rust_obj_))) {}

EMANE::Models::BentPipe::TransponderConfiguration& EMANE::Models::BentPipe::TransponderConfiguration::operator=(const TransponderConfiguration& other) {
  if(this != &other) {
    transmitProcessTOS_ = other.transmitProcessTOS_;
    transmitSlots_ = other.transmitSlots_;
    if(rust_obj_) rust_bentpipe_tc_free(rust_obj_);
    rust_obj_ = rust_bentpipe_tc_clone(other.rust_obj_);
  }
  return *this;
}

EMANE::Models::BentPipe::TransponderConfiguration::~TransponderConfiguration() {
  if(rust_obj_) rust_bentpipe_tc_free(rust_obj_);
}

void EMANE::Models::BentPipe::TransponderConfiguration::setReceiveFrequencyHz(std::uint64_t u64FrequencyHz) { rust_bentpipe_tc_set_rx_freq(rust_obj_, u64FrequencyHz); }
void EMANE::Models::BentPipe::TransponderConfiguration::setReceiveBandwidthHz(std::uint64_t u64BandwidthHz) { rust_bentpipe_tc_set_rx_bw(rust_obj_, u64BandwidthHz); }
void EMANE::Models::BentPipe::TransponderConfiguration::setReceiveAntennaIndex(AntennaIndex antennaIndex) { rust_bentpipe_tc_set_rx_ant(rust_obj_, antennaIndex); }
void EMANE::Models::BentPipe::TransponderConfiguration::setPCRCurveIndex(PCRCurveIndex index) { rust_bentpipe_tc_set_curve(rust_obj_, index); }
void EMANE::Models::BentPipe::TransponderConfiguration::setReceiveAction(ReceiveAction action) { rust_bentpipe_tc_set_rx_action(rust_obj_, static_cast<uint16_t>(action)); }
void EMANE::Models::BentPipe::TransponderConfiguration::setReceiveEnable(bool bEnable) { rust_bentpipe_tc_set_rx_en(rust_obj_, bEnable); }

void EMANE::Models::BentPipe::TransponderConfiguration::setTransmitFrequencyHz(std::uint64_t u64FrequencyHz) { rust_bentpipe_tc_set_tx_freq(rust_obj_, u64FrequencyHz); }
void EMANE::Models::BentPipe::TransponderConfiguration::setTransmitBandwidthHz(std::uint64_t u64BandwidthHz) { rust_bentpipe_tc_set_tx_bw(rust_obj_, u64BandwidthHz); }
void EMANE::Models::BentPipe::TransponderConfiguration::setTransmitDataRatebps(std::uint64_t u64DataRatebps) { rust_bentpipe_tc_set_tx_rate(rust_obj_, u64DataRatebps); }
void EMANE::Models::BentPipe::TransponderConfiguration::setTransmitMTUBytes(std::uint64_t u64MTUBytes) { rust_bentpipe_tc_set_tx_mtu(rust_obj_, u64MTUBytes); }
void EMANE::Models::BentPipe::TransponderConfiguration::setTransmitAntennaIndex(AntennaIndex antennaIndex) { rust_bentpipe_tc_set_tx_ant(rust_obj_, antennaIndex); }
void EMANE::Models::BentPipe::TransponderConfiguration::setTransmitPowerdBm(double dPowerdBm) { rust_bentpipe_tc_set_tx_pwr(rust_obj_, dPowerdBm); }
void EMANE::Models::BentPipe::TransponderConfiguration::setTransmitUbendDelay(const Microseconds & delay) { rust_bentpipe_tc_set_tx_delay(rust_obj_, delay.count()); }
void EMANE::Models::BentPipe::TransponderConfiguration::setTransmitProcessTOS(const TOSSet & tosSet) { transmitProcessTOS_ = tosSet; }
void EMANE::Models::BentPipe::TransponderConfiguration::setTransmitSlotsPerFrame(std::uint16_t u16TransmitSlotsPerFrame) { rust_bentpipe_tc_set_tx_slots_per_frame(rust_obj_, u16TransmitSlotsPerFrame); }
void EMANE::Models::BentPipe::TransponderConfiguration::setTransmitSlotSize(const Microseconds & slotSize) { rust_bentpipe_tc_set_tx_slot_size(rust_obj_, slotSize.count()); }
void EMANE::Models::BentPipe::TransponderConfiguration::setTransmitSlots(const TransmitSlots & slots) { transmitSlots_ = slots; }
void EMANE::Models::BentPipe::TransponderConfiguration::setTransmitEnable(bool bEnable) { rust_bentpipe_tc_set_tx_en(rust_obj_, bEnable); }

EMANE::Models::BentPipe::TransponderIndex EMANE::Models::BentPipe::TransponderConfiguration::getTransponderIndex() const { return rust_bentpipe_tc_get_index(rust_obj_); }
std::uint64_t EMANE::Models::BentPipe::TransponderConfiguration::getReceiveFrequencyHz() const { return rust_bentpipe_tc_get_rx_freq(rust_obj_); }
std::uint64_t EMANE::Models::BentPipe::TransponderConfiguration::getReceiveBandwidthHz() const { return rust_bentpipe_tc_get_rx_bw(rust_obj_); }
EMANE::AntennaIndex EMANE::Models::BentPipe::TransponderConfiguration::getReceiveAntennaIndex() const { return rust_bentpipe_tc_get_rx_ant(rust_obj_); }
EMANE::Models::BentPipe::PCRCurveIndex EMANE::Models::BentPipe::TransponderConfiguration::getPCRCurveIndex() const { return rust_bentpipe_tc_get_curve(rust_obj_); }
EMANE::Models::BentPipe::ReceiveAction EMANE::Models::BentPipe::TransponderConfiguration::getReceiveAction() const { return static_cast<ReceiveAction>(rust_bentpipe_tc_get_rx_action(rust_obj_)); }
bool EMANE::Models::BentPipe::TransponderConfiguration::getReceiveEnable() const { return rust_bentpipe_tc_get_rx_en(rust_obj_); }
std::uint64_t EMANE::Models::BentPipe::TransponderConfiguration::getTransmitFrequencyHz() const { return rust_bentpipe_tc_get_tx_freq(rust_obj_); }
std::uint64_t EMANE::Models::BentPipe::TransponderConfiguration::getTransmitBandwidthHz() const { return rust_bentpipe_tc_get_tx_bw(rust_obj_); }
std::uint64_t EMANE::Models::BentPipe::TransponderConfiguration::getTransmitDataRatebps() const { return rust_bentpipe_tc_get_tx_rate(rust_obj_); }
std::uint64_t EMANE::Models::BentPipe::TransponderConfiguration::getTransmitMTUBytes() const { return rust_bentpipe_tc_get_tx_mtu(rust_obj_); }
EMANE::AntennaIndex EMANE::Models::BentPipe::TransponderConfiguration::getTransmitAntennaIndex() const { return rust_bentpipe_tc_get_tx_ant(rust_obj_); }
double EMANE::Models::BentPipe::TransponderConfiguration::getTransmitPowerdBm() const { return rust_bentpipe_tc_get_tx_pwr(rust_obj_); }
const EMANE::Microseconds & EMANE::Models::BentPipe::TransponderConfiguration::getTransmitUbendDelay() const {
  static thread_local Microseconds us; 
  us = Microseconds{rust_bentpipe_tc_get_tx_delay(rust_obj_)}; 
  return us;
}
const EMANE::Models::BentPipe::TOSSet & EMANE::Models::BentPipe::TransponderConfiguration::getTransmitProcessTOS() const { return transmitProcessTOS_; }
std::uint16_t EMANE::Models::BentPipe::TransponderConfiguration::getTransmitSlotsPerFrame() const { return rust_bentpipe_tc_get_tx_slots_per_frame(rust_obj_); }
const EMANE::Microseconds & EMANE::Models::BentPipe::TransponderConfiguration::getTransmitSlotSize() const {
  static thread_local Microseconds us; 
  us = Microseconds{rust_bentpipe_tc_get_tx_slot_size(rust_obj_)};
  return us;
}
const EMANE::Models::BentPipe::TransmitSlots & EMANE::Models::BentPipe::TransponderConfiguration::getTransmitSlots() const { return transmitSlots_; }
bool EMANE::Models::BentPipe::TransponderConfiguration::getTransmitEnable() const { return rust_bentpipe_tc_get_tx_en(rust_obj_); }
