
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
