with open("src/libemane/mimoreceivepropertiescontrolmessageformatter.cc", "w") as f:
    f.write("""#include <cstdint>
/*
 * Copyright (c) 2020-2021 - Adjacent Link LLC, Bridgewater, New Jersey
 * All rights reserved.
 */

#include "emane/controls/mimoreceivepropertiescontrolmessageformatter.h"

extern "C" {
    void emane_rs_format_mimo_receive_properties(double sot, int64_t prop_delay, void* ctx, void (*add_string)(void*, const char*));
    void emane_rs_format_mimo_receive_antenna_info(uint16_t rx_antenna_index, uint16_t tx_antenna_index, int64_t span, double rx_sens, void* ctx, void (*add_string)(void*, const char*));
    void emane_rs_format_frequency_control_segment(uint64_t freq, int64_t duration, int64_t offset, double power, void* ctx, void (*add_string)(void*, const char*));
    void emane_rs_format_mimo_doppler_shift(uint64_t freq, int64_t shift, void* ctx, void (*add_string)(void*, const char*));
}

EMANE::Controls::MIMOReceivePropertiesControlMessageFormatter::
MIMOReceivePropertiesControlMessageFormatter(const MIMOReceivePropertiesControlMessage * pMsg):
  pMsg_{pMsg}{}

EMANE::Strings EMANE::Controls::MIMOReceivePropertiesControlMessageFormatter::operator()() const
{
  Strings strings;
  
  auto add_str = [](void* ctx, const char* s) {
      static_cast<Strings*>(ctx)->push_back(s);
  };
  
  emane_rs_format_mimo_receive_properties(
      std::chrono::duration_cast<DoubleSeconds>(pMsg_->getTxTime().time_since_epoch()).count(),
      pMsg_->getPropagationDelay().count(),
      &strings,
      add_str
  );

  for(const auto & antennaReceiveInfo :  pMsg_->getAntennaReceiveInfos())
    {
      emane_rs_format_mimo_receive_antenna_info(
          antennaReceiveInfo.getRxAntennaIndex(),
          antennaReceiveInfo.getTxAntennaIndex(),
          antennaReceiveInfo.getSpan().count(),
          antennaReceiveInfo.getReceiverSensitivitydBm(),
          &strings,
          add_str
      );

      for(const auto & segment : antennaReceiveInfo.getFrequencySegments())
        {
          emane_rs_format_frequency_control_segment(
              segment.getFrequencyHz(),
              segment.getDuration().count(),
              segment.getOffset().count(),
              segment.getRxPowerdBm(),
              &strings,
              add_str
          );
        }
    }

  for(const auto & shift : pMsg_->getDopplerShifts())
    {
      emane_rs_format_mimo_doppler_shift(
          shift.first,
          shift.second,
          &strings,
          add_str
      );
    }

  return strings;
}
""")
