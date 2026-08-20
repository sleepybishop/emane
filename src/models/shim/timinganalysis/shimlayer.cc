/*
 * Copyright (c) 2013-2014 - Adjacent Link LLC, Bridgewater, New Jersey
 * Copyright (c) 2009 - DRS CenGen, LLC, Columbia, Maryland
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
 * * Neither the name of DRS CenGen, LLC nor the names of its
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

#include "shimlayer.h"
#include "shimheadermessage.h"
#include "emane/configureexception.h"

namespace
{
  EMANE::ControlMessages clone(const EMANE::ControlMessages & msgs)
  {
    EMANE::ControlMessages clones;

    for(const auto & msg : msgs)
    {
      clones.push_back(msg->clone()); 
    }

    return clones;
  }
}


EMANE::Models::TimingAnalysis::ShimLayer::ShimLayer(NEMId id,
                                                    PlatformServiceProvider * pPlatformService,
                                                    RadioServiceProvider * pRadioService) :
  ShimLayerImplementor{id,pPlatformService,pRadioService},
  u32MaxQueueSize_{0},
  u16PacketId_{0}
{ }

EMANE::Models::TimingAnalysis::ShimLayer::~ShimLayer() 
{ }

void EMANE::Models::TimingAnalysis::ShimLayer::initialize(Registrar & registrar) 
{
  LOGGER_STANDARD_LOGGING(pPlatformService_->logService(),
                          DEBUG_LEVEL,
                          "SHIM %03hu TimingAnalysis::ShimLayer::%s",
                          id_, 
                          __func__);

  auto & configRegistrar = registrar.configurationRegistrar();
  
  configRegistrar.registerNumeric<std::uint32_t>("maxqueuesize",
                                                 ConfigurationProperties::DEFAULT,
                                                 {0},
                                                 "Max size of the delta time storage queue.");

}

void EMANE::Models::TimingAnalysis::ShimLayer::configure(const ConfigurationUpdate & update)
{
  LOGGER_STANDARD_LOGGING(pPlatformService_->logService(),
                          DEBUG_LEVEL,
                          "SHIM %03hu TimingAnalysis::ShimLayer::%s",
                          id_,
                          __func__);

  for(const auto & item : update)
    {
      if(item.first == "maxqueuesize")
        {
          u32MaxQueueSize_ = item.second[0].asUINT32();
          
          LOGGER_STANDARD_LOGGING(pPlatformService_->logService(),
                                  INFO_LEVEL,
                                  "SHIM %03hu TimingAnalysis::ShimLayer::%s,%s = %u",
                                  id_,
                                  __func__,
                                  item.first.c_str(),
                                  u32MaxQueueSize_);
        }
      else
        {
          throw makeException<ConfigureException>("TimingAnalysis::ShimLayer: "
                                                  "Unexpected configuration item %s",
                                                  item.first.c_str());

        }
    }
}

void EMANE::Models::TimingAnalysis::ShimLayer::start()
{
  LOGGER_STANDARD_LOGGING(pPlatformService_->logService(),
                          DEBUG_LEVEL,
                          "SHIM %03hu TimingAnalysis::ShimLayer::%s",
                          id_,
                          __func__);
}


extern "C" void emane_timinganalysis_stop(uint16_t id);
void EMANE::Models::TimingAnalysis::ShimLayer::stop()
{
  emane_timinganalysis_stop(id_);
}
DECLARE_SHIM_LAYER(EMANE::Models::TimingAnalysis::ShimLayer);
