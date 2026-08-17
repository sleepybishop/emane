#include <cstdint>
/*
 * Copyright (c) 2015-2016 - Adjacent Link LLC, Bridgewater, New Jersey
 * All rights reserved.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions
 * are met:
 *
 * * Redistributions of source code must retain the above copyright
 *   notice, this list of conditions and the following disclaimer.
 * * Redistributions in binary form must reproduce the above copyright
 *   notice, this list of conditions and the following disclaimer in
 *   the documentation and/or other materials provided with the
 *   distribution.
 * * Neither the name of Adjacent Link LLC nor the names of its
 *   contributors may be used to endorse or promote products derived
 *   from this software without specific prior written permission.
 *
 * THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS
 * "AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT
 * LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS
 * FOR A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE
 * COPYRIGHT OWNER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT,
 * INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING,
 * BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES;
 * LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER
 * CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT
 * LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN
 * ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE
 * POSSIBILITY OF SUCH DAMAGE.
 */

#include "emane/events/tdmascheduleevent.h"

extern "C" {
    struct EmaneRsTdmaSlotTx {
        bool has_frequency_hz;
        uint64_t frequency_hz;
        bool has_data_rate_bps;
        uint64_t data_rate_bps;
        bool has_service_class;
        uint32_t service_class;
        bool has_power_dbm;
        double power_dbm;
        bool has_destination;
        uint32_t destination;
    };

    struct EmaneRsTdmaSlotRx {
        bool has_frequency_hz;
        uint64_t frequency_hz;
    };

    struct EmaneRsTdmaSlot {
        uint32_t index;
        int32_t type; // SLOT_TX = 1, SLOT_RX = 2, SLOT_IDLE = 3
        bool has_tx;
        EmaneRsTdmaSlotTx tx;
        bool has_rx;
        EmaneRsTdmaSlotRx rx;
    };

    struct EmaneRsTdmaFrame {
        uint32_t index;
        bool has_frequency_hz;
        uint64_t frequency_hz;
        bool has_data_rate_bps;
        uint64_t data_rate_bps;
        bool has_service_class;
        uint32_t service_class;
        bool has_power_dbm;
        double power_dbm;
        EmaneRsTdmaSlot* slots;
        size_t num_slots;
    };

    struct EmaneRsTdmaStructure {
        uint32_t slots_per_frame;
        uint32_t frames_per_multi_frame;
        uint64_t slot_duration_microseconds;
        uint64_t slot_overhead_microseconds;
        uint64_t bandwidth_hz;
    };

    struct EmaneRsTdmaSchedule {
        EmaneRsTdmaFrame* frames;
        size_t num_frames;
        bool has_structure;
        EmaneRsTdmaStructure structure;
        bool has_frequency_hz;
        uint64_t frequency_hz;
        bool has_data_rate_bps;
        uint64_t data_rate_bps;
        bool has_service_class;
        uint32_t service_class;
        bool has_power_dbm;
        double power_dbm;
    };

    bool emane_rs_tdmaschedule_event_deserialize(
        const uint8_t* buf,
        size_t len,
        EmaneRsTdmaSchedule** out_msg
    );

    void emane_rs_tdmaschedule_event_free_deserialize(EmaneRsTdmaSchedule* ptr);
}


#include <cstdint>
#include <tuple>

class EMANE::Events::TDMAScheduleEvent::Implementation
{
public:
  Implementation(const Serialization & serialization):
    bHasStructure_{}
  {
    EmaneRsTdmaSchedule* msg_ptr = nullptr;
    if(!emane_rs_tdmaschedule_event_deserialize(
        reinterpret_cast<const uint8_t*>(serialization.c_str()),
        serialization.size(),
        &msg_ptr))
      {
        throw SerializationException("unable to deserialize : TDMAScheduleEvent");
      }
    
    // We will use a reference to make replacement easier
    const EmaneRsTdmaSchedule& msg = *msg_ptr;

    std::uint32_t u32FramesPerMultiFrame{};
    std::uint32_t u32SlotsPerFrame{};

    if(msg.has_structure)
      {
        const auto & structure = msg.structure;

        u32FramesPerMultiFrame = structure.frames_per_multi_frame;
        u32SlotsPerFrame = structure.slots_per_frame;

        structure_ = SlotStructure{structure.bandwidth_hz,
                                   u32FramesPerMultiFrame,
                                   u32SlotsPerFrame,
                                   Microseconds{structure.slot_duration_microseconds},
                                   Microseconds{structure.slot_overhead_microseconds}};

        bHasStructure_ = true;

        slotInfos_.reserve(u32FramesPerMultiFrame * u32SlotsPerFrame);
      }

    if(bHasStructure_)
      {
        // pre-built a complete schedule defaulting all slots to idle
        for(unsigned i =  0; i < u32FramesPerMultiFrame; ++i)
          {
            for(unsigned j =  0; j < u32SlotsPerFrame; ++j)
              {
                slotInfos_.push_back({SlotInfo::Type::IDLE,i,j});
              }
          }
      }

    for(size_t _f = 0; _f < msg.num_frames; ++_f)
      {
        const auto& frame = msg.frames[_f];
        std::uint32_t u32FrameIndex = frame.index;

        if(bHasStructure_ && u32FrameIndex >= u32FramesPerMultiFrame)
          {
            throw makeException<SerializationException>("TDMAScheduleEvent : Frame %lu index out of range",
                                                        u32FrameIndex);
          }

        std::set<std::uint32_t> presentSlots{};

        for(size_t _s = 0; _s < frame.num_slots; ++_s)
          {
            SlotInfo::Type type{SlotInfo::Type::IDLE};
            std::uint64_t u64FrequencyHz{};
            std::uint64_t u64DataRatebps{};
            std::uint8_t u8ServiceClass{};
            double dPowerdBm{};
            NEMId destination{};

            const auto& slot = frame.slots[_s];
            std::uint32_t u32SlotIndex = slot.index;

            if(bHasStructure_ && u32SlotIndex >= u32SlotsPerFrame)
              {
                 throw makeException<SerializationException>("TDMAScheduleEvent : Frame %lu Slot %lu slot index out of range",
                                                             u32FrameIndex,
                                                             u32SlotIndex);
              }

            presentSlots.insert(u32SlotIndex);

            switch(slot.type)
              {
              case 1:
                {
                  const auto & tx = slot.tx;

                  type = SlotInfo::Type::TX;

                  if(tx.has_frequency_hz)
                    {
                      u64FrequencyHz = tx.frequency_hz;
                    }
                  else if(frame.has_frequency_hz)
                    {
                      u64FrequencyHz = frame.frequency_hz;
                    }
                  else if(msg.has_frequency_hz)
                    {
                      u64FrequencyHz = msg.frequency_hz;
                    }
                  else
                    {
                      throw makeException<SerializationException>("TDMAScheduleEvent : Frame %lu Slot %lu has undeterminable frequency",
                                                                  u32FrameIndex,
                                                                  u32SlotIndex);
                    }

                  if(tx.has_data_rate_bps)
                    {
                      u64DataRatebps = tx.data_rate_bps;
                    }
                  else if(frame.has_data_rate_bps)
                    {
                      u64DataRatebps = frame.data_rate_bps;
                    }
                  else if(msg.has_data_rate_bps)
                    {
                      u64DataRatebps = msg.data_rate_bps;
                    }
                  else
                    {
                      throw makeException<SerializationException>("TDMAScheduleEvent : Frame %lu Slot %lu has undeterminable datarate",
                                                                  u32FrameIndex,
                                                                  u32SlotIndex);
                    }


                  if(tx.has_service_class)
                    {
                      u8ServiceClass = tx.service_class;
                    }
                  else if(frame.has_service_class)
                    {
                      u8ServiceClass = frame.service_class;
                    }
                  else if(msg.has_service_class)
                    {
                      u8ServiceClass = msg.service_class;
                    }
                  else
                    {
                      throw makeException<SerializationException>("TDMAScheduleEvent : Frame %lu Slot %lu has undeterminable class",
                                                                  u32FrameIndex,
                                                                  u32SlotIndex);
                    }


                  if(tx.has_power_dbm)
                    {
                      dPowerdBm = tx.power_dbm;
                    }
                  else if(frame.has_power_dbm)
                    {
                      dPowerdBm = frame.power_dbm;
                    }
                  else if(msg.has_power_dbm)
                    {
                      dPowerdBm = msg.power_dbm;
                    }
                  else
                    {
                      throw makeException<SerializationException>("TDMAScheduleEvent : Frame %lu Slot %lu has undeterminable power",
                                                                  u32FrameIndex,
                                                                  u32SlotIndex);
                    }


                  if(tx.has_destination)
                    {
                      destination = tx.destination;
                    }
                }
                break;

              case 2:
                {
                  const auto & rx = slot.rx;

                  type = SlotInfo::Type::RX;

                  if(rx.has_frequency_hz)
                    {
                      u64FrequencyHz = rx.frequency_hz;
                    }
                  else if(frame.has_frequency_hz)
                    {
                      u64FrequencyHz = frame.frequency_hz;
                    }
                  else if(msg.has_frequency_hz)
                    {
                      u64FrequencyHz = msg.frequency_hz;
                    }
                  else
                    {
                      throw makeException<SerializationException>("TDMAScheduleEvent : Frame %lu Slot %lu has undeterminable frequency",
                                                                  u32FrameIndex,
                                                                  u32SlotIndex);
                    }
                }
                break;

              case 3:
                type = SlotInfo::Type::IDLE;
                break;
              }


            frequencies_.insert(u64FrequencyHz);

            if(bHasStructure_)
              {
                slotInfos_[u32FrameIndex * u32SlotsPerFrame + u32SlotIndex] =
                  {type,
                   u32FrameIndex,
                   u32SlotIndex,
                   u64FrequencyHz,
                   u64DataRatebps,
                   u8ServiceClass,
                   dPowerdBm,
                   destination};
              }
            else
              {
                slotInfos_.push_back({type,
                      u32FrameIndex,
                      u32SlotIndex,
                      u64FrequencyHz,
                      u64DataRatebps,
                      u8ServiceClass,
                      dPowerdBm,
                      destination});
              }
          }

        if(bHasStructure_)
          {
            // add RX slot info for any missing slot
            for(unsigned i = 0; i < u32SlotsPerFrame; ++i)
              {
                if(!presentSlots.count(i))
                  {
                    std::uint64_t u64FrequencyHz{};

                    if(frame.has_frequency_hz)
                      {
                        u64FrequencyHz = frame.frequency_hz;
                      }
                    else if(msg.has_frequency_hz)
                      {
                        u64FrequencyHz = msg.frequency_hz;
                      }
                    else
                      {
                        throw makeException<SerializationException>("TDMAScheduleEvent : Frame %lu Slot %lu has undeterminable frequency",
                                                                    u32FrameIndex,
                                                                    i);
                      }

                    frequencies_.insert(u64FrequencyHz);

                    slotInfos_[u32FrameIndex * u32SlotsPerFrame + i] = {SlotInfo::Type::RX,
                                                                        u32FrameIndex,
                                                                        i,
                                                                        u64FrequencyHz};
                  }
              }
          }
      }

    emane_rs_tdmaschedule_event_free_deserialize(msg_ptr);
  }

  const SlotInfos & getSlotInfos() const
  {
    return slotInfos_;
  }

  const Frequencies & getFrequencies() const
  {
    return frequencies_;
  }

  std::pair<const SlotStructure &,bool> getSlotStructure() const
  {
    return {structure_,bHasStructure_};
  }

private:
  SlotInfos slotInfos_;
  Frequencies frequencies_;
  SlotStructure structure_;
  bool bHasStructure_;
};


EMANE::Events::TDMAScheduleEvent::TDMAScheduleEvent(const Serialization & serialization):
  Event{IDENTIFIER},
  pImpl_{new Implementation{serialization}}{}

EMANE::Events::TDMAScheduleEvent::~TDMAScheduleEvent(){}

const EMANE::Events::SlotInfos & EMANE::Events::TDMAScheduleEvent::getSlotInfos() const
{
  return pImpl_->getSlotInfos();
}


const EMANE::Events::TDMAScheduleEvent::Frequencies &
EMANE::Events::TDMAScheduleEvent::getFrequencies() const
{
  return pImpl_->getFrequencies();
}

std::pair<const EMANE::Events::SlotStructure &,bool>
EMANE::Events::TDMAScheduleEvent::getSlotStructure() const
{
  return  pImpl_->getSlotStructure();
}
