import os

rust_code = """
use std::os::raw::c_void;

#[derive(Clone)]
pub struct FrequencySegmentRs {
    pub frequency_hz: u64,
    pub rx_power_dbm: f64,
    pub duration_microsec: u64,
    pub offset_microsec: u64,
}

#[derive(Clone)]
pub struct AntennaReceiveInfoRs {
    pub rx_antenna_index: u16,
    pub tx_antenna_index: u16,
    pub span_microsec: u64,
    pub receiver_sensitivity_dbm: f64,
    pub segments: Vec<FrequencySegmentRs>,
}

#[derive(Clone)]
pub struct MimoReceivePropertiesControlMessageRs {
    pub sot_microsec: u64,
    pub propagation_microsec: u64,
    pub infos: Vec<AntennaReceiveInfoRs>,
    pub doppler_shifts: Vec<(u64, i64)>, // (frequency_hz, shift)
}

#[no_mangle]
pub extern "C" fn emane_rs_controls_mimo_rx_props_create(
    sot_microsec: u64,
    propagation_microsec: u64,
) -> *mut c_void {
    let msg = Box::new(MimoReceivePropertiesControlMessageRs {
        sot_microsec,
        propagation_microsec,
        infos: Vec::new(),
        doppler_shifts: Vec::new(),
    });
    Box::into_raw(msg) as *mut c_void
}

#[no_mangle]
pub extern "C" fn emane_rs_controls_mimo_rx_props_add_antenna_info(
    ptr: *mut c_void,
    rx_antenna_index: u16,
    tx_antenna_index: u16,
    span_microsec: u64,
    receiver_sensitivity_dbm: f64,
) -> usize {
    let msg = unsafe { &mut *(ptr as *mut MimoReceivePropertiesControlMessageRs) };
    msg.infos.push(AntennaReceiveInfoRs {
        rx_antenna_index,
        tx_antenna_index,
        span_microsec,
        receiver_sensitivity_dbm,
        segments: Vec::new(),
    });
    msg.infos.len() - 1
}

#[no_mangle]
pub extern "C" fn emane_rs_controls_mimo_rx_props_add_frequency_segment(
    ptr: *mut c_void,
    info_idx: usize,
    frequency_hz: u64,
    rx_power_dbm: f64,
    duration_microsec: u64,
    offset_microsec: u64,
) {
    let msg = unsafe { &mut *(ptr as *mut MimoReceivePropertiesControlMessageRs) };
    if let Some(info) = msg.infos.get_mut(info_idx) {
        info.segments.push(FrequencySegmentRs {
            frequency_hz,
            rx_power_dbm,
            duration_microsec,
            offset_microsec,
        });
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_controls_mimo_rx_props_add_doppler_shift(
    ptr: *mut c_void,
    frequency_hz: u64,
    shift: i64,
) {
    let msg = unsafe { &mut *(ptr as *mut MimoReceivePropertiesControlMessageRs) };
    msg.doppler_shifts.push((frequency_hz, shift));
}

#[no_mangle]
pub extern "C" fn emane_rs_controls_mimo_rx_props_clone(ptr: *const c_void) -> *mut c_void {
    let msg = unsafe { &*(ptr as *const MimoReceivePropertiesControlMessageRs) };
    let cloned = Box::new(msg.clone());
    Box::into_raw(cloned) as *mut c_void
}

#[no_mangle]
pub extern "C" fn emane_rs_controls_mimo_rx_props_destroy(ptr: *mut c_void) {
    if !ptr.is_null() {
        unsafe { let _ = Box::from_raw(ptr as *mut MimoReceivePropertiesControlMessageRs); }
    }
}

// Getters are handled by caching in C++ for now to satisfy the const std::vector& signature.
"""

cpp_code = """
#include <cstdint>
#include "emane/controls/mimoreceivepropertiescontrolmessage.h"

extern "C" {
    void* emane_rs_controls_mimo_rx_props_create(uint64_t sot, uint64_t propagation);
    size_t emane_rs_controls_mimo_rx_props_add_antenna_info(void* ptr, uint16_t rx_idx, uint16_t tx_idx, uint64_t span, double sens);
    void emane_rs_controls_mimo_rx_props_add_frequency_segment(void* ptr, size_t info_idx, uint64_t freq, double rx_power, uint64_t dur, uint64_t off);
    void emane_rs_controls_mimo_rx_props_add_doppler_shift(void* ptr, uint64_t freq, int64_t shift);
    void* emane_rs_controls_mimo_rx_props_clone(const void* ptr);
    void emane_rs_controls_mimo_rx_props_destroy(void* ptr);
}

class EMANE::Controls::MIMOReceivePropertiesControlMessage::Implementation
{
public:
  Implementation(const TimePoint & sot,
                 const Microseconds & propagation,
                 const AntennaReceiveInfos & antennaReceiveInfos,
                 const DopplerShifts & dopplerShifts):
    sot_{sot},
    propagation_{propagation},
    antennaReceiveInfos_{antennaReceiveInfos},
    dopplerShifts_{dopplerShifts}
  {
      init_rust();
  }

  Implementation(const TimePoint & sot,
                 const Microseconds & propagation,
                 AntennaReceiveInfos && antennaReceiveInfos,
                 DopplerShifts && dopplerShifts):
    sot_{sot},
    propagation_{propagation},
    antennaReceiveInfos_(std::move(antennaReceiveInfos)),
    dopplerShifts_{std::move(dopplerShifts)}
  {
      init_rust();
  }

  Implementation(const Implementation& other) :
    sot_{other.sot_},
    propagation_{other.propagation_},
    antennaReceiveInfos_{other.antennaReceiveInfos_},
    dopplerShifts_{other.dopplerShifts_}
  {
      pRsMsg_ = emane_rs_controls_mimo_rx_props_clone(other.pRsMsg_);
  }

  ~Implementation() {
      emane_rs_controls_mimo_rx_props_destroy(pRsMsg_);
  }

  const TimePoint & getTxTime() const { return sot_; }
  const Microseconds & getPropagationDelay() const { return propagation_; }
  const AntennaReceiveInfos & getAntennaReceiveInfos() const { return antennaReceiveInfos_; }
  const DopplerShifts & getDopplerShifts() const { return dopplerShifts_; }

  Implementation* clone() const {
      return new Implementation(*this);
  }

private:
  void init_rust() {
      pRsMsg_ = emane_rs_controls_mimo_rx_props_create(
          std::chrono::duration_cast<std::chrono::microseconds>(sot_.time_since_epoch()).count(),
          propagation_.count()
      );
      for(const auto& info : antennaReceiveInfos_) {
          size_t idx = emane_rs_controls_mimo_rx_props_add_antenna_info(
              pRsMsg_, info.getRxAntennaIndex(), info.getTxAntennaIndex(), info.getSpan().count(), info.getReceiverSensitivitydBm()
          );
          for(const auto& seg : info.getFrequencySegments()) {
              emane_rs_controls_mimo_rx_props_add_frequency_segment(
                  pRsMsg_, idx, seg.getFrequencyHz(), seg.getRxPowerdBm(), seg.getDuration().count(), seg.getOffset().count()
              );
          }
      }
      for(const auto& ds : dopplerShifts_) {
          emane_rs_controls_mimo_rx_props_add_doppler_shift(pRsMsg_, ds.first, ds.second);
      }
  }

  void* pRsMsg_;
  TimePoint sot_;
  Microseconds propagation_;
  AntennaReceiveInfos antennaReceiveInfos_;
  DopplerShifts dopplerShifts_;
};

EMANE::Controls::MIMOReceivePropertiesControlMessage::
MIMOReceivePropertiesControlMessage(const MIMOReceivePropertiesControlMessage & msg):
  ControlMessage{IDENTIFIER},
  pImpl_{msg.pImpl_->clone()}
{}

EMANE::Controls::MIMOReceivePropertiesControlMessage::MIMOReceivePropertiesControlMessage(const TimePoint & sot,
                                                                                          const Microseconds & propagation,
                                                                                          const AntennaReceiveInfos & antennaReceiveInfos,
                                                                                          const DopplerShifts & dopplerShifts):
  ControlMessage{IDENTIFIER},
  pImpl_{new Implementation{sot,propagation,antennaReceiveInfos,dopplerShifts}}{}

EMANE::Controls::MIMOReceivePropertiesControlMessage::MIMOReceivePropertiesControlMessage(const TimePoint & sot,
                                                                                          const Microseconds & propagation,
                                                                                          AntennaReceiveInfos && antennaReceiveInfos,
                                                                                          DopplerShifts && dopplerShifts):
  ControlMessage{IDENTIFIER},
  pImpl_{new Implementation{sot,propagation,std::move(antennaReceiveInfos),std::move(dopplerShifts)}}{}

EMANE::Controls::MIMOReceivePropertiesControlMessage::~MIMOReceivePropertiesControlMessage(){}

EMANE::Controls::MIMOReceivePropertiesControlMessage *
EMANE::Controls::MIMOReceivePropertiesControlMessage::create(const TimePoint & sot,
                                                             const Microseconds & propagation,
                                                             const AntennaReceiveInfos & antennaReceiveInfos,
                                                             const DopplerShifts & dopplerShifts)
{
  return new MIMOReceivePropertiesControlMessage{sot,propagation,antennaReceiveInfos,dopplerShifts};
}

EMANE::Controls::MIMOReceivePropertiesControlMessage *
EMANE::Controls::MIMOReceivePropertiesControlMessage::create(const TimePoint & sot,
                                                             const Microseconds & propagation,
                                                             AntennaReceiveInfos && antennaReceiveInfos,
                                                             DopplerShifts && dopplerShifts)
{
  return new MIMOReceivePropertiesControlMessage{sot,propagation,std::move(antennaReceiveInfos),std::move(dopplerShifts)};
}

const EMANE::Controls::AntennaReceiveInfos &
EMANE::Controls::MIMOReceivePropertiesControlMessage::getAntennaReceiveInfos() const
{
  return pImpl_->getAntennaReceiveInfos();
}

const EMANE::Microseconds &
EMANE::Controls::MIMOReceivePropertiesControlMessage::getPropagationDelay() const
{
  return pImpl_->getPropagationDelay();
}

const EMANE::TimePoint & EMANE::Controls::MIMOReceivePropertiesControlMessage::getTxTime() const
{
  return pImpl_->getTxTime();
}

const EMANE::Controls::DopplerShifts &
EMANE::Controls::MIMOReceivePropertiesControlMessage::getDopplerShifts() const
{
  return pImpl_->getDopplerShifts();
}

EMANE::Controls::MIMOReceivePropertiesControlMessage *
EMANE::Controls::MIMOReceivePropertiesControlMessage::clone() const
{
  return new MIMOReceivePropertiesControlMessage{*this};
}
"""

with open("rust/emane-core/src/controls/mimo_receive_properties_control_message.rs", "w") as f:
    f.write(rust_code)

with open("src/libemane/mimoreceivepropertiescontrolmessage.cc", "w") as f:
    f.write(cpp_code)
