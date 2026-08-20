/*
 * Copyright (c) 2023 - Adjacent Link LLC, Bridgewater, New Jersey
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


#include "transpondernoprotocol.h"
#include "transponderuser.h"

extern "C" {
  void* rust_bentpipe_tnp_new();
  void rust_bentpipe_tnp_free(void*);
  void rust_bentpipe_tnp_start(void*);
  void rust_bentpipe_tnp_stop(void*);
  bool rust_bentpipe_tnp_is_tx_opp(void*, uint64_t now_us);
  void rust_bentpipe_tnp_prepare_tx(void*, uint64_t now_us, size_t len, uint64_t rate, uint64_t* out_duration, uint64_t* out_idx);
}

EMANE::Models::BentPipe::TransponderNoProtocol::TransponderNoProtocol(NEMId id,
                                                                      PlatformServiceProvider * pPlatformService,
                                                                      TransponderUser * pTransponderUser,
                                                                      const TransponderConfiguration & transponderConfiguration):
  Transponder{id,pPlatformService,pTransponderUser,transponderConfiguration},
  eot_{},
  u64TxOpportunityIndex_{},
  rust_obj_{rust_bentpipe_tnp_new()} {}

EMANE::Models::BentPipe::TransponderNoProtocol::~TransponderNoProtocol() {
  if (rust_obj_) rust_bentpipe_tnp_free(rust_obj_);
}

void EMANE::Models::BentPipe::TransponderNoProtocol::start() {
  rust_bentpipe_tnp_start(rust_obj_);
}

void EMANE::Models::BentPipe::TransponderNoProtocol::stop() {
  rust_bentpipe_tnp_stop(rust_obj_);
}

bool EMANE::Models::BentPipe::TransponderNoProtocol::isTransmitOpportunity(const TimePoint & now) {
  uint64_t now_us = std::chrono::duration_cast<Microseconds>(now.time_since_epoch()).count();
  return rust_bentpipe_tnp_is_tx_opp(rust_obj_, now_us);
}

size_t EMANE::Models::BentPipe::TransponderNoProtocol::getMTUBytes() const {
  return configuration_.getTransmitMTUBytes();
}

EMANE::Models::BentPipe::Transponder::TransmissionInfo
EMANE::Models::BentPipe::TransponderNoProtocol::prepareTransmission(const TimePoint & now,
                                                                    size_t lengthBytes,
                                                                    MessageComponents && components)
{
  uint64_t now_us = std::chrono::duration_cast<Microseconds>(now.time_since_epoch()).count();
  uint64_t out_dur = 0;
  uint64_t out_idx = 0;
  rust_bentpipe_tnp_prepare_tx(rust_obj_, now_us, lengthBytes, configuration_.getTransmitDataRatebps(), &out_dur, &out_idx);
  
  u64TxOpportunityIndex_ = out_idx;
  Microseconds durationMicroseconds{out_dur};
  eot_ = now + durationMicroseconds;

  // set a timer for the next tx opporunity
  pPlatformService_->timerService().
    schedule(std::bind(&TransponderNoProtocol::processTxOpportunity,
                       this,
                       u64TxOpportunityIndex_),
             eot_);

  return {BentPipeMessage{now,
                          configuration_.getPCRCurveIndex(),
                          std::move(components)},
          durationMicroseconds};
}

void EMANE::Models::BentPipe::TransponderNoProtocol::processTxOpportunity(std::uint64_t u64TxOpportunityIndex)
{
  if(u64TxOpportunityIndex == u64TxOpportunityIndex_)
    {
      pTransponderUser_->notifyTxOpportunity(this);
    }
}
