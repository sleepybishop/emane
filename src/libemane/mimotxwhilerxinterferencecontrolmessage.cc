
#include <cstdint>
#include "emane/controls/mimotxwhilerxinterferencecontrolmessage.h"

extern "C" {
    void* emane_rs_controls_mimo_tx_rx_create();
    size_t emane_rs_controls_mimo_tx_rx_add_frequency_group(void* ptr);
    void emane_rs_controls_mimo_tx_rx_add_frequency_segment(void* ptr, size_t group_idx, uint64_t freq, double power, uint64_t dur, uint64_t off);
    size_t emane_rs_controls_mimo_tx_rx_add_rx_antenna(void* ptr, uint16_t antenna_index);
    size_t emane_rs_controls_mimo_tx_rx_add_interference(void* ptr, size_t map_idx, size_t freq_group_idx);
    void emane_rs_controls_mimo_tx_rx_add_interference_power(void* ptr, size_t map_idx, size_t interference_idx, double power_mw);
    void* emane_rs_controls_mimo_tx_rx_clone(const void* ptr);
    void emane_rs_controls_mimo_tx_rx_destroy(void* ptr);
}

class EMANE::Controls::MIMOTxWhileRxInterferenceControlMessage::Implementation
{
public:
  Implementation(const FrequencyGroups & frequencyGroups,
                 const RxAntennaInterferenceMap & rxAntennaSelections):
    frequencyGroups_{frequencyGroups},
    rxAntennaSelections_{rxAntennaSelections}
  {
      init_rust();
  }

  Implementation(FrequencyGroups && frequencyGroups,
                 const RxAntennaInterferenceMap & rxAntennaSelections):
    frequencyGroups_{std::move(frequencyGroups)},
    rxAntennaSelections_{rxAntennaSelections}
  {
      init_rust();
  }

  Implementation(FrequencyGroups && frequencyGroups,
                 RxAntennaInterferenceMap && rxAntennaSelections):
    frequencyGroups_{std::move(frequencyGroups)},
    rxAntennaSelections_{std::move(rxAntennaSelections)}
  {
      init_rust();
  }

  Implementation(const Implementation& other) :
    frequencyGroups_{other.frequencyGroups_},
    rxAntennaSelections_{other.rxAntennaSelections_}
  {
      pRsMsg_ = emane_rs_controls_mimo_tx_rx_clone(other.pRsMsg_);
  }

  ~Implementation() {
      emane_rs_controls_mimo_tx_rx_destroy(pRsMsg_);
  }

  const FrequencyGroups & getFrequencyGroups() const { return frequencyGroups_; }
  const RxAntennaInterferenceMap & getRxAntennaInterferenceMap() const { return rxAntennaSelections_; }

  Implementation* clone() const {
      return new Implementation(*this);
  }

private:
  void init_rust() {
      pRsMsg_ = emane_rs_controls_mimo_tx_rx_create();
      for(const auto& group : frequencyGroups_) {
          size_t idx = emane_rs_controls_mimo_tx_rx_add_frequency_group(pRsMsg_);
          for(const auto& seg : group) {
              emane_rs_controls_mimo_tx_rx_add_frequency_segment(
                  pRsMsg_, idx, seg.getFrequencyHz(), seg.getPowerdBm().first, seg.getDuration().count(), seg.getOffset().count()
              );
          }
      }
      for(const auto& map_entry : rxAntennaSelections_) {
          size_t map_idx = emane_rs_controls_mimo_tx_rx_add_rx_antenna(pRsMsg_, map_entry.first);
          for(const auto& interference : map_entry.second) {
              size_t interference_idx = emane_rs_controls_mimo_tx_rx_add_interference(
                  pRsMsg_, map_idx, interference.getFrequencyGroupIndex()
              );
              for(const auto& power_mw : interference.getPowerMilliWatts()) {
                  emane_rs_controls_mimo_tx_rx_add_interference_power(pRsMsg_, map_idx, interference_idx, power_mw);
              }
          }
      }
  }

  void* pRsMsg_;
  FrequencyGroups frequencyGroups_;
  RxAntennaInterferenceMap rxAntennaSelections_;
};

EMANE::Controls::MIMOTxWhileRxInterferenceControlMessage::
MIMOTxWhileRxInterferenceControlMessage(const MIMOTxWhileRxInterferenceControlMessage & msg):
  ControlMessage{IDENTIFIER},
  pImpl_{msg.pImpl_->clone()}
{}

EMANE::Controls::MIMOTxWhileRxInterferenceControlMessage::MIMOTxWhileRxInterferenceControlMessage(const FrequencyGroups & frequencyGroups,
                                                                                                  const RxAntennaInterferenceMap & rxAntennaSelections):
  ControlMessage{IDENTIFIER},
  pImpl_{new Implementation{frequencyGroups,rxAntennaSelections}}{}

EMANE::Controls::MIMOTxWhileRxInterferenceControlMessage::MIMOTxWhileRxInterferenceControlMessage(FrequencyGroups && frequencyGroups,
                                                                                                  const RxAntennaInterferenceMap & rxAntennaSelections):
  ControlMessage{IDENTIFIER},
  pImpl_{new Implementation{std::move(frequencyGroups),rxAntennaSelections}}{}

EMANE::Controls::MIMOTxWhileRxInterferenceControlMessage::MIMOTxWhileRxInterferenceControlMessage(FrequencyGroups && frequencyGroups,
                                                                                                  RxAntennaInterferenceMap && rxAntennaSelections):
  ControlMessage{IDENTIFIER},
  pImpl_{new Implementation{std::move(frequencyGroups),std::move(rxAntennaSelections)}}{}

EMANE::Controls::MIMOTxWhileRxInterferenceControlMessage::~MIMOTxWhileRxInterferenceControlMessage(){}

EMANE::Controls::MIMOTxWhileRxInterferenceControlMessage *
EMANE::Controls::MIMOTxWhileRxInterferenceControlMessage::create(const FrequencyGroups & frequencyGroups,
                                                                 const RxAntennaInterferenceMap & rxAntennaSelections)
{
  return new MIMOTxWhileRxInterferenceControlMessage{frequencyGroups,rxAntennaSelections};
}

EMANE::Controls::MIMOTxWhileRxInterferenceControlMessage *
EMANE::Controls::MIMOTxWhileRxInterferenceControlMessage::create(FrequencyGroups && frequencyGroups,
                                                                 const RxAntennaInterferenceMap & rxAntennaSelections)
{
  return new MIMOTxWhileRxInterferenceControlMessage{std::move(frequencyGroups),rxAntennaSelections};
}

EMANE::Controls::MIMOTxWhileRxInterferenceControlMessage *
EMANE::Controls::MIMOTxWhileRxInterferenceControlMessage::create(FrequencyGroups && frequencyGroups,
                                                                 RxAntennaInterferenceMap && rxAntennaSelections)
{
  return new MIMOTxWhileRxInterferenceControlMessage{std::move(frequencyGroups),
                                                       std::move(rxAntennaSelections)};
}

const EMANE::FrequencyGroups &
EMANE::Controls::MIMOTxWhileRxInterferenceControlMessage::getFrequencyGroups() const
{
  return pImpl_->getFrequencyGroups();
}

const EMANE::Controls::MIMOTxWhileRxInterferenceControlMessage::RxAntennaInterferenceMap &
EMANE::Controls::MIMOTxWhileRxInterferenceControlMessage::getRxAntennaInterferenceMap() const
{
  return pImpl_->getRxAntennaInterferenceMap();
}

EMANE::Controls::MIMOTxWhileRxInterferenceControlMessage *
EMANE::Controls::MIMOTxWhileRxInterferenceControlMessage::clone() const
{
  return new MIMOTxWhileRxInterferenceControlMessage{*this};
}
