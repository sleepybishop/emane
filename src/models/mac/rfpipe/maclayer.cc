/*
 * Copyright (c) 2013-2017,2023 - Adjacent Link LLC, Bridgewater,
 *  New Jersey
 * Copyright (c) 2008-2009 - DRS CenGen, LLC, Columbia, Maryland
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

#define  _GLIBCXX_USE_NANOSLEEP

#include "maclayer.h"
#include "rfpipemacheadermessage.h"

#include "emane/controls/flowcontrolcontrolmessage.h"
#include "emane/controls/receivepropertiescontrolmessage.h"
#include "emane/controls/frequencycontrolmessage.h"
#include "emane/controls/serializedcontrolmessage.h"
#include "emane/controls/r2riselfmetriccontrolmessage.h"
#include "emane/controls/receivepropertiescontrolmessageformatter.h"
#include "emane/controls/frequencycontrolmessageformatter.h"
#include "emane/controls/timestampcontrolmessage.h"

#include "emane/spectrumserviceexception.h"
#include "emane/configureexception.h"
#include "emane/utils/conversionutils.h"
#include "emane/utils/spectrumwindowutils.h"

#include <sstream>

namespace
{
  const char * pzLayerName{"RFPipeMACLayer"};

  const std::uint16_t DROP_CODE_SINR               = 1;
  const std::uint16_t DROP_CODE_REGISTRATION_ID    = 2;
  const std::uint16_t DROP_CODE_DST_MAC            = 3;
  const std::uint16_t DROP_CODE_QUEUE_OVERFLOW     = 4;
  const std::uint16_t DROP_CODE_BAD_CONTROL_INFO   = 5;
  const std::uint16_t DROP_CODE_BAD_SPECTRUM_QUERY = 6;
  const std::uint16_t DROP_CODE_FLOW_CONTROL_ERROR = 7;

  EMANE::StatisticTableLabels STATISTIC_TABLE_LABELS
    {
      "SINR",
      "Reg Id",
      "Dst MAC",
      "Queue Overflow",
      "Bad Control",
      "Bad Spectrum Query",
      "Flow Control"
    };
}

EMANE::Models::RFPipe::MACLayer::MACLayer(NEMId id,
                                          PlatformServiceProvider * pPlatformServiceProvider,
                                          RadioServiceProvider * pRadioServiceProvider):
  MACLayerImplementor{id, pPlatformServiceProvider, pRadioServiceProvider},
  rs_state_{emane_rs_rfpipe_mac_new(id)},
  u64TxSequenceNumber_{},
  radioMetricTimedEventId_{},
  commonLayerStatistics_{STATISTIC_TABLE_LABELS,{},"0"},
  RNDZeroToOne_{0.0f, 1.0f},
  pRNDJitter_{},
  fJitterSeconds_{},
  pNumDownstreamQueueDelay_{},
  downstreamQueueTimedEventId_{},
  bHasPendingDownstreamQueueEntry_{},
  pendingDownstreamQueueEntry_{},
  currentEndOfTransmissionTime_{}
{}

EMANE::Models::RFPipe::MACLayer::~MACLayer() {
  emane_rs_rfpipe_mac_free(rs_state_);
}


void
EMANE::Models::RFPipe::MACLayer::initialize(Registrar & registrar)
{
  LOGGER_STANDARD_LOGGING(pPlatformService_->logService(),
                          DEBUG_LEVEL,
                          "MACI %03hu %s::%s",
                          id_,
                          pzLayerName,
                          __func__);


  auto & configRegistrar = registrar.configurationRegistrar();

  configRegistrar.registerNumeric<bool>("enablepromiscuousmode",
                                        ConfigurationProperties::DEFAULT |
                                        ConfigurationProperties::MODIFIABLE,
                                        {false},
                                        "Defines whether promiscuous mode is enabled or not."
                                        " If promiscuous mode is enabled, all received packets"
                                        " (intended for the given node or not) that pass the"
                                        " probability of reception check are sent upstream to"
                                        " the transport.");

  configRegistrar.registerNumeric<std::uint64_t>("datarate",
                                                 ConfigurationProperties::DEFAULT |
                                                 ConfigurationProperties::MODIFIABLE,
                                                 {1000000},
                                                 "Defines the transmit datarate in bps."
                                                 " The datarate is used by the transmitter"
                                                 " to compute the transmit delay (packet size/datarate)"
                                                 " between successive transmissions.",
                                                 1);

  /** [configurationregistrar-registernumeric-snippet] */

  configRegistrar.registerNumeric<float>("jitter",
                                         ConfigurationProperties::DEFAULT |
                                         ConfigurationProperties::MODIFIABLE,
                                         {0},
                                         "Defines delay jitter in seconds applied to each transmitted packet."
                                         " The jitter is added to the configured delay based on a uniform"
                                         " random distribution between +/- the configured jitter value.",
                                         0.0f);

  configRegistrar.registerNumeric<float>("delay",
                                         ConfigurationProperties::DEFAULT |
                                         ConfigurationProperties::MODIFIABLE,
                                         {0},
                                         "Defines an additional fixed delay in seconds applied to each"
                                         " transmitted packet.",
                                         0.0f);

  configRegistrar.registerNumeric<bool>("flowcontrolenable",
                                        ConfigurationProperties::DEFAULT,
                                        {false},
                                        "Defines whether flow control is enabled. Flow control only works"
                                        " with the virtual transport and the setting must match the setting"
                                        " within the virtual transport configuration.");

  configRegistrar.registerNumeric<std::uint16_t>("flowcontroltokens",
                                                 ConfigurationProperties::DEFAULT,
                                                 {10},
                                                 "Defines the maximum number of flow control tokens"
                                                 " (packet transmission units) that can be processed from the"
                                                 " virtual transport without being refreshed. The number of"
                                                 " available tokens at any given time is coordinated with the"
                                                 " virtual transport and when the token count reaches zero, no"
                                                 " further packets are transmitted causing application socket"
                                                 " queues to backup.");
  /** [configurationregistrar-registernumeric-snippet] */

  /** [configurationregistrar-registernonnumeric-snippet] */
  configRegistrar.registerNonNumeric<std::string>("pcrcurveuri",
                                                  ConfigurationProperties::REQUIRED,
                                                  {},
                                                  "Defines the URI of the Packet Completion Rate (PCR) curve"
                                                  " file. The PCR curve file contains probability of reception curves"
                                                  " as a function of Signal to Interference plus Noise Ratio (SINR).");
  /** [configurationregistrar-registernonnumeric-snippet] */

  configRegistrar.registerNumeric<bool>("radiometricenable",
                                        ConfigurationProperties::DEFAULT,
                                        {false},
                                        "Defines if radio metrics will be reported up via the Radio to Router Interface"
                                        " (R2RI).");


  configRegistrar.registerNumeric<float>("radiometricreportinterval",
                                         ConfigurationProperties::DEFAULT,
                                         {1.0f},
                                         "Defines the metric report interval in seconds in support of the R2RI feature.",
                                         0.1f,
                                         60.0f);

  configRegistrar.registerNumeric<float>("neighbormetricdeletetime",
                                         ConfigurationProperties::DEFAULT |
                                         ConfigurationProperties::MODIFIABLE,
                                         {60.0f},
                                         "Defines the time in seconds of no RF receptions from a given neighbor"
                                         " before it is removed from the neighbor table.",
                                         1.0f,
                                         3660.0f);


  auto & statisticRegistrar = registrar.statisticRegistrar();

  commonLayerStatistics_.registerStatistics(statisticRegistrar);

  downstreamQueue_.registerStatistics(statisticRegistrar);

  pNumDownstreamQueueDelay_ =
    statisticRegistrar.registerNumeric<std::uint64_t>("numDownstreamQueueDelay",
                                                      StatisticProperties::CLEARABLE,
                                                      "Accumulation of downstream queue delay in microseconds.");

  avgDownstreamQueueDelay_.registerStatistic(
                                             statisticRegistrar.registerNumeric<float>("avgDownstreamQueueDelay",
                                                                                       StatisticProperties::CLEARABLE));

  // rust handles metric managers
}

void
EMANE::Models::RFPipe::MACLayer::configure(const ConfigurationUpdate & update)
{
  LOGGER_STANDARD_LOGGING(pPlatformService_->logService(),
                          DEBUG_LEVEL,
                          "MACI %03hu %s::%s",
                          id_,
                          pzLayerName,
                          __func__);

  ConfigurationUpdate rfReceiveMetricTableConfiguration{};

  for(const auto & item : update)
    {
      if(item.first == "enablepromiscuousmode")
        {
          /** [logservice-infolog-snippet] */
          bPromiscuousMode_ = item.second[0].asBool();

          LOGGER_STANDARD_LOGGING(pPlatformService_->logService(),
                                  INFO_LEVEL,
                                  "MACI %03hu %s::%s %s = %s",
                                  id_,
                                  pzLayerName,
                                  __func__,
                                  item.first.c_str(),
                                  bPromiscuousMode_ ? "on" : "off");
          /** [logservice-infolog-snippet] */
        }
      else if(item.first == "datarate")
        {
          u64DataRatebps_ = item.second[0].asUINT64();

          LOGGER_STANDARD_LOGGING(pPlatformService_->logService(),
                                  INFO_LEVEL,
                                  "MACI %03hu %s::%s %s = %ju",
                                  id_,
                                  pzLayerName,
                                  __func__,
                                  item.first.c_str(),
                                  u64DataRatebps_);
        }
      else if(item.first == "jitter")
        {
          fJitterSeconds_ = item.second[0].asFloat();

          if(fJitterSeconds_ > 0.0f)
            {
              // create a random mumber distrubtion +- the jitter range
              pRNDJitter_.reset(new Utils::RandomNumberDistribution<std::mt19937,
                                std::uniform_real_distribution<float>>
                                (-fJitterSeconds_ / 2.0f, fJitterSeconds_ / 2.0));
            }

          LOGGER_STANDARD_LOGGING(pPlatformService_->logService(),
                                  INFO_LEVEL,
                                  "MACI %03hu %s::%s %s = %f",
                                  id_,
                                  pzLayerName,
                                  __func__,
                                  item.first.c_str(),
                                  fJitterSeconds_);
        }
      else if(item.first == "delay")
        {
          delayMicroseconds_ =
            std::chrono::duration_cast<Microseconds>(DoubleSeconds{item.second[0].asFloat()});

          LOGGER_STANDARD_LOGGING(pPlatformService_->logService(),
                                  INFO_LEVEL,
                                  "MACI %03hu %s::%s %s = %lf",
                                  id_,
                                  pzLayerName,
                                  __func__,
                                  item.first.c_str(),
                                  std::chrono::duration_cast<DoubleSeconds>(delayMicroseconds_).count());

        }
      else if(item.first == "flowcontrolenable")
        {
          bFlowControlEnable_ = item.second[0].asBool();

          LOGGER_STANDARD_LOGGING(pPlatformService_->logService(),
                                  INFO_LEVEL,
                                  "MACI %03hu %s::%s %s = %s",
                                  id_,
                                  pzLayerName,
                                  __func__,
                                  item.first.c_str(),
                                  bFlowControlEnable_ ? "on" : "off");
        }
      else if(item.first == "flowcontroltokens")
        {
          u16FlowControlTokens_ = item.second[0].asUINT16();

          LOGGER_STANDARD_LOGGING(pPlatformService_->logService(),
                                  INFO_LEVEL,
                                  "MACI %03hu %s::%s %s = %hu",
                                  id_,
                                  pzLayerName,
                                  __func__,
                                  item.first.c_str(),
                                  u16FlowControlTokens_);
        }
      else if(item.first == "pcrcurveuri")
        {
          sPCRCurveURI_ = item.second[0].asString();

          LOGGER_STANDARD_LOGGING(pPlatformService_->logService(),
                                  INFO_LEVEL,
                                  "MACI %03hu %s::%s %s = %s",
                                  id_,
                                  pzLayerName,
                                  __func__,
                                  item.first.c_str(),
                                  sPCRCurveURI_.c_str());
        }
      else if(item.first == "radiometricenable")
        {
          bRadioMetricEnable_ = item.second[0].asBool();

          LOGGER_STANDARD_LOGGING(pPlatformService_->logService(),
                                  INFO_LEVEL,
                                  "MACI %03hu %s::%s %s = %s",
                                  id_,
                                  pzLayerName,
                                  __func__,
                                  item.first.c_str(),
                                  bRadioMetricEnable_ ? "on" : "off");
        }
      else if(item.first == "radiometricreportinterval")
        {
          radioMetricReportIntervalMicroseconds_ =
            std::chrono::duration_cast<Microseconds>(DoubleSeconds{item.second[0].asFloat()});

          LOGGER_STANDARD_LOGGING(pPlatformService_->logService(),
                                  INFO_LEVEL,
                                  "MACI %03hu %s::%s %s = %lf",
                                  id_,
                                  pzLayerName,
                                  __func__,
                                  item.first.c_str(),
                                  std::chrono::duration_cast<DoubleSeconds>(radioMetricReportIntervalMicroseconds_).count());
        }
      else if(item.first == "neighbormetricdeletetime")
        {
          neighborMetricDeleteTimeMicroseconds_ =
            std::chrono::duration_cast<Microseconds>(DoubleSeconds{item.second[0].asFloat()});

          LOGGER_STANDARD_LOGGING(pPlatformService_->logService(),
                                  INFO_LEVEL,
                                  "MACI %03hu %s::%s %s = %lf",
                                  id_,
                                  pzLayerName,
                                  __func__,
                                  item.first.c_str(),
                                  std::chrono::duration_cast<DoubleSeconds>(neighborMetricDeleteTimeMicroseconds_).count());
        }
      else
        {
          const std::string rfSignalTableConfigPrefix{"rfsignaltable."};
          if(item.first.compare(0,rfSignalTableConfigPrefix.size(),rfSignalTableConfigPrefix) != 0)
            {
              throw makeException<ConfigureException>("RFPipe::MACLayer: "
                                                      "Unexpected configuration item %s",
                                                      item.first.c_str());
            }
        }
    }

  const char* pcrURI = "";
  if(!sPCRCurveURI_.empty()) pcrURI = sPCRCurveURI_.c_str();
  emane_rs_rfpipe_mac_configure(rs_state_,
      bPromiscuousMode_,
      u64DataRatebps_,
      delayMicroseconds_.count(),
      bFlowControlEnable_,
      bRadioMetricEnable_,
      u16FlowControlTokens_,
      pcrURI,
      radioMetricReportIntervalMicroseconds_.count(),
      neighborMetricDeleteTimeMicroseconds_.count());
}


void
EMANE::Models::RFPipe::MACLayer::start()
{
  LOGGER_STANDARD_LOGGING(pPlatformService_->logService(),
                          DEBUG_LEVEL,
                          "MACI %03hu %s::%s",
                          id_,
                          pzLayerName,
                          __func__);

  emane_rs_rfpipe_mac_start(rs_state_);
}


void
EMANE::Models::RFPipe::MACLayer::postStart()
{
  LOGGER_STANDARD_LOGGING(pPlatformService_->logService(),
                          DEBUG_LEVEL,
                          "MACI %03hu %s::%s",
                          id_,
                          pzLayerName,
                          __func__);

  // check flow control enabled
  if(bFlowControlEnable_)
    {
      // flow control start handled by Rust FFI
      LOGGER_STANDARD_LOGGING(pPlatformService_->logService(),
                              DEBUG_LEVEL,
                              "MACI %03hu %s::%s sent a flow control token update,"
                              " a handshake response is required to process packets",
                              id_,
                              pzLayerName,
                              __func__);
    }

  // set the timer timeout (absolute time), arg, interval
  /** [timerservice-scheduletimedevent-1-snippet] */
  radioMetricTimedEventId_ =
    pPlatformService_->timerService().
    schedule([this](const TimePoint &,
                    const TimePoint &,
                    const TimePoint &)
    {
      if(!bRadioMetricEnable_)
        {
          // neighborMetricManager_.updateNeighborStatus(); handled by rust
        }
      else
        {
          ControlMessages msgs{
            static_cast<const ControlMessage*>(Controls::R2RISelfMetricControlMessage::create(u64DataRatebps_,
                                                                                              u64DataRatebps_,
                                                                                              radioMetricReportIntervalMicroseconds_))};

          sendUpstreamControl(msgs);
        }
    },
             Clock::now() + radioMetricReportIntervalMicroseconds_,
             radioMetricReportIntervalMicroseconds_);
  /** [timerservice-scheduletimedevent-1-snippet] */

  LOGGER_STANDARD_LOGGING(pPlatformService_->logService(),
                          DEBUG_LEVEL,
                          "MACI %03hu %s::%s: added radio metric timed eventId %zu",
                          id_,
                          pzLayerName,
                          __func__,
                          radioMetricTimedEventId_);
}


void
EMANE::Models::RFPipe::MACLayer::processConfiguration(const ConfigurationUpdate & update)
{
  LOGGER_STANDARD_LOGGING(pPlatformService_->logService(),
                          DEBUG_LEVEL,
                          "MACI %03hu %s::%s",
                          id_,
                          pzLayerName,
                          __func__);

  for(const auto & item : update)
    {
      if(item.first == "enablepromiscuousmode")
        {
          bPromiscuousMode_ = item.second[0].asBool();

          LOGGER_STANDARD_LOGGING(pPlatformService_->logService(),
                                  INFO_LEVEL,
                                  "MACI %03hu %s::%s %s = %s",
                                  id_,
                                  pzLayerName,
                                  __func__,
                                  item.first.c_str(),
                                  bPromiscuousMode_ ? "on" : "off");

        }
      else if(item.first == "datarate")
        {
          u64DataRatebps_ = item.second[0].asUINT64();

          LOGGER_STANDARD_LOGGING(pPlatformService_->logService(),
                                  INFO_LEVEL,
                                  "MACI %03hu %s::%s %s = %ju",
                                  id_,
                                  pzLayerName,
                                  __func__,
                                  item.first.c_str(),
                                  u64DataRatebps_);
        }
      else if(item.first == "jitter")
        {
          fJitterSeconds_ = item.second[0].asFloat();

          if(fJitterSeconds_ > 0.0f)
            {
              // create a random mumber distrubtion +- the jitter range
              pRNDJitter_.reset(new Utils::RandomNumberDistribution<std::mt19937,
                                std::uniform_real_distribution<float>>
                                (-fJitterSeconds_ / 2.0f, fJitterSeconds_ / 2.0));
            }

          LOGGER_STANDARD_LOGGING(pPlatformService_->logService(),
                                  INFO_LEVEL,
                                  "MACI %03hu %s::%s %s = %f",
                                  id_,
                                  pzLayerName,
                                  __func__,
                                  item.first.c_str(),
                                  fJitterSeconds_);
        }
      else if(item.first == "delay")
        {
          delayMicroseconds_ =
            std::chrono::duration_cast<Microseconds>(DoubleSeconds{item.second[0].asFloat()});

          LOGGER_STANDARD_LOGGING(pPlatformService_->logService(),
                                  INFO_LEVEL,
                                  "MACI %03hu %s::%s %s = %lf",
                                  id_,
                                  pzLayerName,
                                  __func__,
                                  item.first.c_str(),
                                  std::chrono::duration_cast<DoubleSeconds>(delayMicroseconds_).count());

        }
      else if(item.first == "neighbormetricdeletetime")
        {
          neighborMetricDeleteTimeMicroseconds_ =
            std::chrono::duration_cast<Microseconds>(DoubleSeconds{item.second[0].asFloat()});

          // neighborMetricManager_.setNeighborDeleteTimeMicroseconds(neighborMetricDeleteTimeMicroseconds_); // handled by rust

          LOGGER_STANDARD_LOGGING(pPlatformService_->logService(),
                                  INFO_LEVEL,
                                  "MACI %03hu %s::%s %s = %lf",
                                  id_,
                                  pzLayerName,
                                  __func__,
                                  item.first.c_str(),
                                  std::chrono::duration_cast<DoubleSeconds>(neighborMetricDeleteTimeMicroseconds_).count());
        }
      else
        {
          throw makeException<ConfigureException>("RFPipe::MACLayer: "
                                                  "Unexpected configuration item %s",
                                                  item.first.c_str());
        }
    }
}



void
EMANE::Models::RFPipe::MACLayer::stop()
{
  LOGGER_STANDARD_LOGGING(pPlatformService_->logService(),
                          DEBUG_LEVEL,
                          "MACI %03hu %s::%s",
                          id_,
                          pzLayerName,
                          __func__);

  /** [timerservice-canceltimedevent-snippet] */
  pPlatformService_->timerService().cancelTimedEvent(downstreamQueueTimedEventId_);
  /** [timerservice-canceltimedevent-snippet] */

  downstreamQueueTimedEventId_ = 0;

  // check flow control enabled
  if(bFlowControlEnable_)
    {
      // flowControlManager_.stop(); handled by rust
    }
}



void
EMANE::Models::RFPipe::MACLayer::destroy()
  throw()
{
  LOGGER_STANDARD_LOGGING(pPlatformService_->logService(),
                          DEBUG_LEVEL,
                          "MACI %03hu %s::%s",
                          id_,
                          pzLayerName,
                          __func__);
}


void
EMANE::Models::RFPipe::MACLayer::processUpstreamControl(const ControlMessages &){}


void
EMANE::Models::RFPipe::MACLayer::processDownstreamControl(const ControlMessages & msgs)
{
  for(const auto & pMessage : msgs)
    {
      LOGGER_STANDARD_LOGGING(pPlatformService_->logService(),
                              DEBUG_LEVEL,
                              "MACI %03hu %s::%s downstream control message id %hu",
                              id_,
                              pzLayerName,
                              __func__,
                              pMessage->getId());

      switch(pMessage->getId())
        {
        case Controls::FlowControlControlMessage::IDENTIFIER:
          {
            const auto pFlowControlControlMessage =
              static_cast<const Controls::FlowControlControlMessage *>(pMessage);

            if(bFlowControlEnable_)
              {
                LOGGER_STANDARD_LOGGING(pPlatformService_->logService(),
                                        DEBUG_LEVEL,
                                        "MACI %03hu %s::%s received a flow control token request/response",
                                        id_,
                                        pzLayerName,
                                        __func__);

                emane_rs_rfpipe_mac_process_flow_control_message(rs_state_, pFlowControlControlMessage->getTokens());
              }
            else
              {
                LOGGER_STANDARD_LOGGING(pPlatformService_->logService(),
                                        ERROR_LEVEL,
                                        "MACI %03hu %s::%s received a flow control token request but"
                                        " flow control is not enabled",
                                        id_,
                                        pzLayerName,
                                        __func__);
              }
          }
          break;

          /** [serializedcontrolmessage-flowcontrol-snibbet] */
        case Controls::SerializedControlMessage::IDENTIFIER:
          {
            const auto pSerializedControlMessage =
              static_cast<const Controls::SerializedControlMessage *>(pMessage);

            switch(pSerializedControlMessage->getSerializedId())
              {
              case Controls::FlowControlControlMessage::IDENTIFIER:
                {
                  std::unique_ptr<Controls::FlowControlControlMessage>
                    pFlowControlControlMessage{
                    Controls::FlowControlControlMessage::create(pSerializedControlMessage->getSerialization())};

                  if(bFlowControlEnable_)
                    {
                      emane_rs_rfpipe_mac_process_flow_control_message(rs_state_, pFlowControlControlMessage->getTokens());
                    }
                  else
                    {
                      LOGGER_STANDARD_LOGGING(pPlatformService_->logService(),
                                              ERROR_LEVEL,
                                              "MACI %03hu %s::%s received a flow control token request but"
                                              " flow control is not enabled",
                                              id_,
                                              pzLayerName,
                                              __func__);
                    }
                }
                break;
              }
          }
          /** [serializedcontrolmessage-flowcontrol-snibbet] */
        }
    }
}


void
EMANE::Models::RFPipe::MACLayer::processUpstreamPacket(const CommonMACHeader & commonMACHeader,
                                                       UpstreamPacket & pkt,
                                                       const ControlMessages & msgs)
{
  // get current time
  TimePoint beginTime{Clock::now()};

  commonLayerStatistics_.processInbound(pkt);

  if(commonMACHeader.getRegistrationId() != type_)
    {
      LOGGER_STANDARD_LOGGING(pPlatformService_->logService(),
                              ERROR_LEVEL,
                              "MACI %03hu %s::%s: MAC Registration Id %hu does not match our Id %hu, drop.",
                              id_,
                              pzLayerName,
                              __func__,
                              commonMACHeader.getRegistrationId(),
                              type_);

      commonLayerStatistics_.processOutbound(pkt,
                                             std::chrono::duration_cast<Microseconds>(Clock::now() - beginTime),
                                             DROP_CODE_REGISTRATION_ID);

      // drop
      return;
    }

  size_t len{pkt.stripLengthPrefixFraming()};

  const PacketInfo & pktInfo{pkt.getPacketInfo()};

  if(len && pkt.length() >= len)
    {
      MACHeaderMessage rfpipeMACHeader{pkt.get(), len};

      pkt.strip(len);

      const Controls::ReceivePropertiesControlMessage * pReceivePropertiesControlMessage{};

      const Controls::FrequencyControlMessage * pFrequencyControlMessage{};

      for(auto & pControlMessage : msgs)
        {
          switch(pControlMessage->getId())
            {
            case EMANE::Controls::ReceivePropertiesControlMessage::IDENTIFIER:
              {
                /** [logservice-loggerfnvargs-snippet] */
                pReceivePropertiesControlMessage =
                  static_cast<const Controls::ReceivePropertiesControlMessage *>(pControlMessage);

                LOGGER_VERBOSE_LOGGING_FN_VARGS(pPlatformService_->logService(),
                                                DEBUG_LEVEL,
                                                Controls::ReceivePropertiesControlMessageFormatter(pReceivePropertiesControlMessage),
                                                "MACI %03hu RFPipe::%s Receiver Properties Control Message",
                                                id_,
                                                __func__);
                /** [logservice-loggerfnvargs-snippet] */
              }
              break;

            case Controls::FrequencyControlMessage::IDENTIFIER:
              {
                pFrequencyControlMessage =
                  static_cast<const Controls::FrequencyControlMessage *>(pControlMessage);

                LOGGER_VERBOSE_LOGGING_FN_VARGS(pPlatformService_->logService(),
                                                DEBUG_LEVEL,
                                                Controls::FrequencyControlMessageFormatter(pFrequencyControlMessage),
                                                "MACI %03hu RFPipe::%s Frequency Control Message",
                                                id_,
                                                __func__);

              }

              break;
            }
        }


      if(!pReceivePropertiesControlMessage || !pFrequencyControlMessage ||
         pFrequencyControlMessage->getFrequencySegments().empty())
        {
          LOGGER_STANDARD_LOGGING(pPlatformService_->logService(),
                                  ERROR_LEVEL,
                                  "MACI %03hu %s::%s: phy control "
                                  "message not provided from src %hu, drop",
                                  id_,
                                  pzLayerName,
                                  __func__,
                                  pktInfo.getSource());

          commonLayerStatistics_.processOutbound(pkt,
                                                 std::chrono::duration_cast<Microseconds>(Clock::now() - beginTime),
                                                 DROP_CODE_BAD_CONTROL_INFO);

          // drop
          return;
        }
      else
        {
          const auto & frequencySegments = pFrequencyControlMessage->getFrequencySegments();

          /** [startofreception-calculation-snibbet] */
          TimePoint startOfReception{pReceivePropertiesControlMessage->getTxTime() +
                                     pReceivePropertiesControlMessage->getPropagationDelay() +
                                     frequencySegments.begin()->getOffset()};
          /** [startofreception-calculation-snibbet] */

          Microseconds span{pReceivePropertiesControlMessage->getSpan()};

          auto callback =
            std::bind([this,
                       startOfReception,
                       frequencySegments,
                       span,
                       beginTime](UpstreamPacket & pkt,
                                  std::uint64_t u64SequenceNumber,
                                  std::uint64_t u64DataRate)
            {
              const PacketInfo & pktInfo{pkt.getPacketInfo()};

              const FrequencySegment & frequencySegment{*frequencySegments.begin()};

              double dSINR{};
              double dNoiseFloordB{};
              double dReceiverSensitivitymW{};

              try
                {
                  auto window = pRadioService_->spectrumService().request(frequencySegment.getFrequencyHz(),
                                                                          span,
                                                                          startOfReception);

                  bool bSignalInNoise{};
                  std::tie(dNoiseFloordB,bSignalInNoise) =
                    Utils::maxBinNoiseFloor(window,frequencySegment.getRxPowerdBm());

                  dSINR = frequencySegment.getRxPowerdBm() - dNoiseFloordB;

                  dReceiverSensitivitymW = std::get<3>(window);
                }
              catch(SpectrumServiceException & exp)
                {
                  commonLayerStatistics_.processOutbound(pkt,
                                                         std::chrono::duration_cast<Microseconds>(Clock::now() - beginTime),
                                                         DROP_CODE_BAD_SPECTRUM_QUERY);
                  // drop
                  return;
                }

              auto action = emane_rs_rfpipe_mac_process_upstream(
                  rs_state_,
                  dSINR,
                  pkt.length(),
                  pktInfo.getSource(),
                  pktInfo.getDestination(),
                  u64SequenceNumber,
                  dNoiseFloordB,
                  std::chrono::duration_cast<Microseconds>(startOfReception.time_since_epoch()).count(),
                  frequencySegment.getDuration().count(),
                  u64DataRate,
                  frequencySegment.getFrequencyHz(),
                  frequencySegment.getRxPowerdBm(),
                  Utils::MILLIWATT_TO_DB(dReceiverSensitivitymW)
              );

              if (action.action == 0) { // drop
                  commonLayerStatistics_.processOutbound(pkt,
                                                         std::chrono::duration_cast<Microseconds>(Clock::now() - beginTime),
                                                         action.drop_code);
                  return;
              } else { // forward upstream
                  commonLayerStatistics_.processOutbound(pkt,
                                                         std::chrono::duration_cast<Microseconds>(Clock::now() - beginTime));
                  sendUpstreamPacket(pkt);
                  return;
              }
            },pkt,commonMACHeader.getSequenceNumber(),rfpipeMACHeader.getDataRate());


          auto eor = startOfReception + frequencySegments.begin()->getDuration();

          if(eor > beginTime)
            {
              // wait for end of reception to complete processing
              pPlatformService_->timerService().schedule(callback,eor);
            }
          else
            {
              // we can process now, end of reception has past
              callback();
            }
        }
    }
}




void
EMANE::Models::RFPipe::MACLayer::processDownstreamPacket(DownstreamPacket & pkt,
                                                         const ControlMessages &)
{
  TimePoint beginTime{Clock::now()};

  commonLayerStatistics_.processInbound(pkt);

  auto action = emane_rs_rfpipe_mac_process_downstream(rs_state_, pkt.length());
  if (action.action == 0) {
      commonLayerStatistics_.processOutbound(pkt,
                                             std::chrono::duration_cast<Microseconds>(Clock::now() - beginTime),
                                             action.drop_code);
      return;
  }

  Microseconds durationMicroseconds{action.duration_microseconds};
  DownstreamQueueEntry entry{pkt,
                             beginTime,
                             durationMicroseconds,
                             u64DataRatebps_};

  Microseconds currentDelay{action.delay_microseconds + getJitter().count()};

  // if we are delaying the transmission, we will delay adding it
  // to the downstream queue
  if(currentDelay > Microseconds::zero())
    {
      /** [timerservice-scheduletimedevent-2-snippet] */
      pPlatformService_->timerService().
        schedule([this,
                  entry=std::move(entry)]
                 (const TimePoint &,
                  const TimePoint &,
                  const TimePoint &) mutable
        {
          pendOrEnqueueDownstreamQueueEntry(std::move(entry));
        },
                 beginTime + currentDelay);
      /** [timerservice-scheduletimedevent-2-snippet] */
    }
  else
    {
      pendOrEnqueueDownstreamQueueEntry(std::move(entry));
    }
}

void EMANE::Models::RFPipe::MACLayer::pendOrEnqueueDownstreamQueueEntry(DownstreamQueueEntry && entry)
{
  auto now = Clock::now();

  if(bHasPendingDownstreamQueueEntry_)
    {
      std::vector<DownstreamQueueEntry> result{downstreamQueue_.enqueue(std::move(entry))};

      // check for discarded, update stats
      for(auto & iter : result)
        {
          commonLayerStatistics_.processOutbound(iter.pkt_,
                                                 std::chrono::duration_cast<Microseconds>(Clock::now() - iter.acquireTime_),
                                                 DROP_CODE_QUEUE_OVERFLOW);

          // drop, replace token
          if(bFlowControlEnable_)
            {
              emane_rs_rfpipe_mac_flow_control_add_token(rs_state_);
            }
        }
    }
  else
    {
      bHasPendingDownstreamQueueEntry_ = true;

      pendingDownstreamQueueEntry_ = std::move(entry);
    }

  // there is something in the queue and it is past EoT
  if(currentEndOfTransmissionTime_ <= now)
    {
      sendDownstreamQueueEntry();
    }
}

void
EMANE::Models::RFPipe::MACLayer::sendDownstreamQueueEntry()
{
  auto now = Clock::now();

  if(bHasPendingDownstreamQueueEntry_ && currentEndOfTransmissionTime_ <= now)
    {
      if(bFlowControlEnable_)
        {
          emane_rs_rfpipe_mac_flow_control_add_token(rs_state_);
        }

      MACHeaderMessage rfpipeMACHeader{pendingDownstreamQueueEntry_.u64DataRatebps_};

      Serialization serialization{rfpipeMACHeader.serialize()};

      auto & pkt = pendingDownstreamQueueEntry_.pkt_;

      // prepend mac header to outgoing packet
      pkt.prepend(serialization.c_str(), serialization.size());

      // next prepend the serialization length
      pkt.prependLengthPrefixFraming(serialization.size());

      commonLayerStatistics_.processOutbound(pkt,
                                             std::chrono::duration_cast<Microseconds>(now - pendingDownstreamQueueEntry_.acquireTime_));

      /** [pysicallayer-frequencycontrolmessage-snippet] */

      sendDownstreamPacket(CommonMACHeader(type_, u64TxSequenceNumber_++),
                           pkt,
                           {Controls::FrequencyControlMessage::create(0, // bandwidth (0 means use phy default)
                                                                      {{0, pendingDownstreamQueueEntry_.durationMicroseconds_}}), // freq (0 means use phy default)
                            Controls::TimeStampControlMessage::create(now)});

      /** [pysicallayer-frequencycontrolmessage-snippet] */
      // queue delay
      Microseconds queueDelayMicroseconds{std::chrono::duration_cast<Microseconds>(now - pendingDownstreamQueueEntry_.acquireTime_)};

      *pNumDownstreamQueueDelay_ += queueDelayMicroseconds.count();

      avgDownstreamQueueDelay_.update(queueDelayMicroseconds.count());

      emane_rs_rfpipe_mac_downstream_dequeue(rs_state_, pendingDownstreamQueueEntry_.pkt_.getPacketInfo().getDestination(), queueDelayMicroseconds.count(), downstreamQueue_.getMaxCapacity(), downstreamQueue_.getCurrentDepth(), downstreamQueue_.getNumDiscards(true));

      // earliest you can send the next packet
      currentEndOfTransmissionTime_ =
        now + pendingDownstreamQueueEntry_.durationMicroseconds_;

      std::tie(pendingDownstreamQueueEntry_,
               bHasPendingDownstreamQueueEntry_) =
        downstreamQueue_.dequeue();

      // set the next wake up at EoT to check to see if there is a packet to send
      downstreamQueueTimedEventId_ =
        pPlatformService_->timerService().
        schedule(std::bind(&MACLayer::sendDownstreamQueueEntry,this),
                 currentEndOfTransmissionTime_);
    }
}



EMANE::Microseconds
EMANE::Models::RFPipe::MACLayer::getDurationMicroseconds(size_t lengthInBytes)
{
  if(u64DataRatebps_ > 0)
    {
      float fSeconds{((lengthInBytes * 8.0f) / u64DataRatebps_)};

      return std::chrono::duration_cast<Microseconds>(DoubleSeconds{fSeconds});
    }
  else
    {
      return Microseconds{};
    }
}


EMANE::Microseconds
EMANE::Models::RFPipe::MACLayer::getJitter()
{
  if(fJitterSeconds_ > 0.0f)
    {
      return std::chrono::duration_cast<Microseconds>(DoubleSeconds{(*pRNDJitter_)()});
    }
  else
    {
      return Microseconds{};
    }
}



DECLARE_MAC_LAYER(EMANE::Models::RFPipe::MACLayer);
