#include <cstdint>
/*
 * Copyright (c) 2014 - Adjacent Link LLC, Bridgewater, New Jersey
 * All rights reserved.
 */

#include "emane/events/commeffecteventformatter.h"

extern "C" {
    void emane_rs_format_comm_effect_element(uint16_t nem_id, double latency_sec, double jitter_sec, float prob_loss, float prob_dup, uint64_t unicast_bps, uint64_t broadcast_bps, void* ctx, void (*add_string)(void*, const char*));
}

EMANE::Events::CommEffectEventFormatter::
CommEffectEventFormatter(const CommEffectEvent & event):
  event_(event)
{}
      
EMANE::Strings EMANE::Events::CommEffectEventFormatter::operator()() const
{
  Strings strings;

  for(const auto & effect : event_.getCommEffects())
    {
        emane_rs_format_comm_effect_element(
            effect.getNEMId(),
            std::chrono::duration_cast<DoubleSeconds>(effect.getLatency()).count(),
            std::chrono::duration_cast<DoubleSeconds>(effect.getJitter()).count(),
            effect.getProbabilityLoss(),
            effect.getProbabilityDuplicate(),
            effect.getUnicastBitRate(),
            effect.getBroadcastBitRate(),
            &strings,
            [](void* ctx, const char* s) {
                static_cast<Strings*>(ctx)->push_back(s);
            }
        );
    }

  return strings;
}
