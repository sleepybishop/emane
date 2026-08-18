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
pub struct AntennaRs {
    pub frequency_group_index: usize,
    pub index: u16,
    pub bandwidth_hz: u64,
    pub spectral_mask_index: u16,
}

#[derive(Clone)]
pub struct MimoTransmitPropertiesControlMessageRs {
    pub frequency_groups: Vec<Vec<FrequencySegmentRs>>,
    pub transmit_antennas: Vec<AntennaRs>,
}

#[no_mangle]
pub extern "C" fn emane_rs_controls_mimo_tx_props_create() -> *mut c_void {
    let msg = Box::new(MimoTransmitPropertiesControlMessageRs {
        frequency_groups: Vec::new(),
        transmit_antennas: Vec::new(),
    });
    Box::into_raw(msg) as *mut c_void
}

#[no_mangle]
pub extern "C" fn emane_rs_controls_mimo_tx_props_add_frequency_group(
    ptr: *mut c_void,
) -> usize {
    let msg = unsafe { &mut *(ptr as *mut MimoTransmitPropertiesControlMessageRs) };
    msg.frequency_groups.push(Vec::new());
    msg.frequency_groups.len() - 1
}

#[no_mangle]
pub extern "C" fn emane_rs_controls_mimo_tx_props_add_frequency_segment(
    ptr: *mut c_void,
    group_idx: usize,
    frequency_hz: u64,
    power_dbm: f64,
    duration_microsec: u64,
    offset_microsec: u64,
) {
    let msg = unsafe { &mut *(ptr as *mut MimoTransmitPropertiesControlMessageRs) };
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
pub extern "C" fn emane_rs_controls_mimo_tx_props_add_antenna(
    ptr: *mut c_void,
    frequency_group_index: usize,
    index: u16,
    bandwidth_hz: u64,
    spectral_mask_index: u16,
) {
    let msg = unsafe { &mut *(ptr as *mut MimoTransmitPropertiesControlMessageRs) };
    msg.transmit_antennas.push(AntennaRs {
        frequency_group_index,
        index,
        bandwidth_hz,
        spectral_mask_index,
    });
}

#[no_mangle]
pub extern "C" fn emane_rs_controls_mimo_tx_props_clone(ptr: *const c_void) -> *mut c_void {
    let msg = unsafe { &*(ptr as *const MimoTransmitPropertiesControlMessageRs) };
    let cloned = Box::new(msg.clone());
    Box::into_raw(cloned) as *mut c_void
}

#[no_mangle]
pub extern "C" fn emane_rs_controls_mimo_tx_props_destroy(ptr: *mut c_void) {
    if !ptr.is_null() {
        unsafe { let _ = Box::from_raw(ptr as *mut MimoTransmitPropertiesControlMessageRs); }
    }
}
"""

cpp_code = """
#include <cstdint>
#include "emane/controls/mimotransmitpropertiescontrolmessage.h"

extern "C" {
    void* emane_rs_controls_mimo_tx_props_create();
    size_t emane_rs_controls_mimo_tx_props_add_frequency_group(void* ptr);
    void emane_rs_controls_mimo_tx_props_add_frequency_segment(void* ptr, size_t group_idx, uint64_t freq, double power, uint64_t dur, uint64_t off);
    void emane_rs_controls_mimo_tx_props_add_antenna(void* ptr, size_t fg_idx, uint16_t idx, uint64_t bw, uint16_t sm_idx);
    void* emane_rs_controls_mimo_tx_props_clone(const void* ptr);
    void emane_rs_controls_mimo_tx_props_destroy(void* ptr);
}

class EMANE::Controls::MIMOTransmitPropertiesControlMessage::Implementation
{
public:
  Implementation(const FrequencyGroups & frequencyGroups,
                 const Antennas & transmitAntennas):
    frequencyGroups_{frequencyGroups},
    transmitAntennas_{transmitAntennas}
  {
      init_rust();
  }

  Implementation(FrequencyGroups && frequencyGroups,
                 const Antennas & transmitAntennas):
    frequencyGroups_{std::move(frequencyGroups)},
    transmitAntennas_{transmitAntennas}
  {
      init_rust();
  }

  Implementation(FrequencyGroups && frequencyGroups,
                 Antennas && transmitAntennas):
    frequencyGroups_{std::move(frequencyGroups)},
    transmitAntennas_{std::move(transmitAntennas)}
  {
      init_rust();
  }

  Implementation(const Implementation& other) :
    frequencyGroups_{other.frequencyGroups_},
    transmitAntennas_{other.transmitAntennas_}
  {
      pRsMsg_ = emane_rs_controls_mimo_tx_props_clone(other.pRsMsg_);
  }

  ~Implementation() {
      emane_rs_controls_mimo_tx_props_destroy(pRsMsg_);
  }

  const FrequencyGroups & getFrequencyGroups() const { return frequencyGroups_; }
  const Antennas & getTransmitAntennas() const { return transmitAntennas_; }

  Implementation* clone() const {
      return new Implementation(*this);
  }

private:
  void init_rust() {
      pRsMsg_ = emane_rs_controls_mimo_tx_props_create();
      for(const auto& group : frequencyGroups_) {
          size_t idx = emane_rs_controls_mimo_tx_props_add_frequency_group(pRsMsg_);
          for(const auto& seg : group) {
              emane_rs_controls_mimo_tx_props_add_frequency_segment(
                  pRsMsg_, idx, seg.getFrequencyHz(), seg.getPowerdBm().first, seg.getDuration().count(), seg.getOffset().count()
              );
          }
      }
      for(const auto& ant : transmitAntennas_) {
          emane_rs_controls_mimo_tx_props_add_antenna(
              pRsMsg_, ant.getFrequencyGroupIndex(), ant.getIndex(), ant.getBandwidthHz(), ant.getSpectralMaskIndex()
          );
      }
  }

  void* pRsMsg_;
  FrequencyGroups frequencyGroups_;
  Antennas transmitAntennas_;
};

EMANE::Controls::MIMOTransmitPropertiesControlMessage::
MIMOTransmitPropertiesControlMessage(const MIMOTransmitPropertiesControlMessage & msg):
  ControlMessage{IDENTIFIER},
  pImpl_{msg.pImpl_->clone()}
{}

EMANE::Controls::MIMOTransmitPropertiesControlMessage::MIMOTransmitPropertiesControlMessage(const FrequencyGroups & frequencyGroups,
                                                                                            const Antennas & transmitAntennas):
  ControlMessage{IDENTIFIER},
  pImpl_{new Implementation{frequencyGroups,transmitAntennas}}{}

EMANE::Controls::MIMOTransmitPropertiesControlMessage::MIMOTransmitPropertiesControlMessage(FrequencyGroups && frequencyGroups,
                                                                                            const Antennas & transmitAntennas):
  ControlMessage{IDENTIFIER},
  pImpl_{new Implementation{std::move(frequencyGroups),transmitAntennas}}{}

EMANE::Controls::MIMOTransmitPropertiesControlMessage::MIMOTransmitPropertiesControlMessage(FrequencyGroups && frequencyGroups,
                                                                                            Antennas && transmitAntennas):
  ControlMessage{IDENTIFIER},
  pImpl_{new Implementation{std::move(frequencyGroups),std::move(transmitAntennas)}}{}

EMANE::Controls::MIMOTransmitPropertiesControlMessage::~MIMOTransmitPropertiesControlMessage(){}

EMANE::Controls::MIMOTransmitPropertiesControlMessage *
EMANE::Controls::MIMOTransmitPropertiesControlMessage::create(const FrequencyGroups & frequencyGroups,
                                                              const Antennas & transmitAntennas)
{
  return new MIMOTransmitPropertiesControlMessage{frequencyGroups,transmitAntennas};
}

EMANE::Controls::MIMOTransmitPropertiesControlMessage *
EMANE::Controls::MIMOTransmitPropertiesControlMessage::create(FrequencyGroups && frequencyGroups,
                                                              const Antennas & transmitAntennas)
{
  return new MIMOTransmitPropertiesControlMessage{std::move(frequencyGroups),transmitAntennas};
}

EMANE::Controls::MIMOTransmitPropertiesControlMessage *
EMANE::Controls::MIMOTransmitPropertiesControlMessage::create(FrequencyGroups && frequencyGroups,
                                                              Antennas && transmitAntennas)
{
  return new MIMOTransmitPropertiesControlMessage{std::move(frequencyGroups),
                                                    std::move(transmitAntennas)};
}

const EMANE::FrequencyGroups &
EMANE::Controls::MIMOTransmitPropertiesControlMessage::getFrequencyGroups() const
{
  return pImpl_->getFrequencyGroups();
}

const EMANE::Antennas &
EMANE::Controls::MIMOTransmitPropertiesControlMessage::getTransmitAntennas() const
{
  return pImpl_->getTransmitAntennas();
}

EMANE::Controls::MIMOTransmitPropertiesControlMessage *
EMANE::Controls::MIMOTransmitPropertiesControlMessage::clone() const
{
  return new MIMOTransmitPropertiesControlMessage{*this};
}
"""

with open("rust/emane-core/src/controls/mimo_transmit_properties_control_message.rs", "w") as f:
    f.write(rust_code)

with open("src/libemane/mimotransmitpropertiescontrolmessage.cc", "w") as f:
    f.write(cpp_code)
