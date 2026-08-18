import os

rust_code = """
use std::os::raw::c_void;

#[derive(Clone)]
pub struct FrequencySegmentRs {
    pub frequency_hz: u64,
    pub power_dbm: f64,
    pub duration_microsec: u64,
    pub offset_microsec: u64,
}

#[derive(Clone)]
pub struct AntennaSelfInterferenceRs {
    pub frequency_group_index: usize,
    pub power_dbm: f64,
}

#[derive(Clone)]
pub struct MimoTxWhileRxInterferenceControlMessageRs {
    pub frequency_groups: Vec<Vec<FrequencySegmentRs>>,
    pub rx_antenna_interferences: Vec<(u16, Vec<AntennaSelfInterferenceRs>)>,
}

#[no_mangle]
pub extern "C" fn emane_rs_controls_mimo_tx_rx_create() -> *mut c_void {
    let msg = Box::new(MimoTxWhileRxInterferenceControlMessageRs {
        frequency_groups: Vec::new(),
        rx_antenna_interferences: Vec::new(),
    });
    Box::into_raw(msg) as *mut c_void
}

#[no_mangle]
pub extern "C" fn emane_rs_controls_mimo_tx_rx_add_frequency_group(
    ptr: *mut c_void,
) -> usize {
    let msg = unsafe { &mut *(ptr as *mut MimoTxWhileRxInterferenceControlMessageRs) };
    msg.frequency_groups.push(Vec::new());
    msg.frequency_groups.len() - 1
}

#[no_mangle]
pub extern "C" fn emane_rs_controls_mimo_tx_rx_add_frequency_segment(
    ptr: *mut c_void,
    group_idx: usize,
    frequency_hz: u64,
    power_dbm: f64,
    duration_microsec: u64,
    offset_microsec: u64,
) {
    let msg = unsafe { &mut *(ptr as *mut MimoTxWhileRxInterferenceControlMessageRs) };
    if let Some(group) = msg.frequency_groups.get_mut(group_idx) {
        group.push(FrequencySegmentRs {
            frequency_hz,
            power_dbm,
            duration_microsec,
            offset_microsec,
        });
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_controls_mimo_tx_rx_add_rx_antenna(
    ptr: *mut c_void,
    antenna_index: u16,
) -> usize {
    let msg = unsafe { &mut *(ptr as *mut MimoTxWhileRxInterferenceControlMessageRs) };
    msg.rx_antenna_interferences.push((antenna_index, Vec::new()));
    msg.rx_antenna_interferences.len() - 1
}

#[no_mangle]
pub extern "C" fn emane_rs_controls_mimo_tx_rx_add_interference(
    ptr: *mut c_void,
    map_idx: usize,
    frequency_group_index: usize,
    power_dbm: f64,
) {
    let msg = unsafe { &mut *(ptr as *mut MimoTxWhileRxInterferenceControlMessageRs) };
    if let Some(entry) = msg.rx_antenna_interferences.get_mut(map_idx) {
        entry.1.push(AntennaSelfInterferenceRs {
            frequency_group_index,
            power_dbm,
        });
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_controls_mimo_tx_rx_clone(ptr: *const c_void) -> *mut c_void {
    let msg = unsafe { &*(ptr as *const MimoTxWhileRxInterferenceControlMessageRs) };
    let cloned = Box::new(msg.clone());
    Box::into_raw(cloned) as *mut c_void
}

#[no_mangle]
pub extern "C" fn emane_rs_controls_mimo_tx_rx_destroy(ptr: *mut c_void) {
    if !ptr.is_null() {
        unsafe { let _ = Box::from_raw(ptr as *mut MimoTxWhileRxInterferenceControlMessageRs); }
    }
}
"""

cpp_code = """
#include <cstdint>
#include "emane/controls/mimotxwhilerxinterferencecontrolmessage.h"

extern "C" {
    void* emane_rs_controls_mimo_tx_rx_create();
    size_t emane_rs_controls_mimo_tx_rx_add_frequency_group(void* ptr);
    void emane_rs_controls_mimo_tx_rx_add_frequency_segment(void* ptr, size_t group_idx, uint64_t freq, double power, uint64_t dur, uint64_t off);
    size_t emane_rs_controls_mimo_tx_rx_add_rx_antenna(void* ptr, uint16_t antenna_index);
    void emane_rs_controls_mimo_tx_rx_add_interference(void* ptr, size_t map_idx, size_t freq_group_idx, double power);
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
              emane_rs_controls_mimo_tx_rx_add_interference(
                  pRsMsg_, map_idx, interference.getFrequencyGroupIndex(), interference.getPowerdBm()
              );
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
"""

with open("rust/emane-core/src/controls/mimo_tx_while_rx_interference_control_message.rs", "w") as f:
    f.write(rust_code)

with open("src/libemane/mimotxwhilerxinterferencecontrolmessage.cc", "w") as f:
    f.write(cpp_code)
