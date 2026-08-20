extern "C" {
  void emane_bentpipe_radiomodel_processUpstreamPacket(void* radiomodel, const void* hdr, void* pkt, const void* msgs);
  void emane_bentpipe_radiomodel_processDownstreamPacket(void* radiomodel, void* pkt, const void* msgs);
}
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

#include "radiomodel.h"
#include "configurehelpers.h"
#include "transpondernoprotocol.h"
#include "transpondertdmaprotocol.h"
#include "transponderconfigurationupdate.h"

#include "bentpipemessage.pb.h"

#include "emane/mactypes.h"
#include "emane/configureexception.h"
#include "emane/controls/rxantennaaddcontrolmessage.h"
#include "emane/controls/rxantennaupdatecontrolmessage.h"
#include "emane/controls/rxantennaremovecontrolmessage.h"
#include "emane/controls/timestampcontrolmessage.h"
#include "emane/controls/mimotransmitpropertiescontrolmessage.h"
#include "emane/controls/mimoreceivepropertiescontrolmessage.h"
#include "emane/controls/mimoreceivepropertiescontrolmessageformatter.h"

EMANE::Models::BentPipe::RadioModel::RadioModel(NEMId id,
                                                PlatformServiceProvider * pPlatformServiceProvider,
                                                RadioServiceProvider * pRadioServiceProvider):
  MACLayerImplementor{id, pPlatformServiceProvider, pRadioServiceProvider},
  packetStatusPublisher_{},
  queueManager_{id,pPlatformServiceProvider,&packetStatusPublisher_},
  tosToTransponder_{},
  u64SequenceNumber_{}
{
  tosToTransponder_.fill(-1);
}


EMANE::Models::BentPipe::RadioModel::~RadioModel()
{}


void
EMANE::Models::BentPipe::RadioModel::initialize(Registrar & registrar)
{
  auto & configRegistrar = registrar.configurationRegistrar();

  LOGGER_STANDARD_LOGGING(pPlatformService_->logService(),
                          INFO_LEVEL,
                          "MACI %03hu BendPipe::RadioModel !!! Warning: Make sure"
                          " 'arpcacheenable' is 'off' if using the VirtualTransport !!!",
                          id_);

  LOGGER_STANDARD_LOGGING(pPlatformService_->logService(),
                          DEBUG_LEVEL,
                          "MACI %03hu BendPipe::RadioModel::%s",
                          id_,
                          __func__);

  configRegistrar.registerNonNumeric<std::string>("transponder.receive.frequency",
                                                  ConfigurationProperties::REQUIRED |
                                                  ConfigurationProperties::MODIFIABLE,
                                                  {},
                                                  "Defines per transponder receive center frequency in hz"
                                                  " with the following format: <transponder index>:<frequency hz>.",
                                                  1,
                                                  std::numeric_limits<std::uint16_t>::max(),
                                                  "^\\d+:\\d+(\\.\\d+){0,1}(G|M|K){0,1}$");

  configRegistrar.registerNonNumeric<std::string>("transponder.receive.bandwidth",
                                                  ConfigurationProperties::REQUIRED,
                                                  {},
                                                  "Defines per transponder receive bandwidth in hz"
                                                  " with the following format: <transponder index>:<frequency hz>.",
                                                  1,
                                                  std::numeric_limits<std::uint16_t>::max(),
                                                  "^\\d+:\\d+(\\.\\d+){0,1}(G|M|K){0,1}$");

  configRegistrar.registerNonNumeric<std::string>("transponder.receive.antenna",
                                                  ConfigurationProperties::REQUIRED,
                                                  {},
                                                  "Defines per transponder receive antenna"
                                                  " with the following format:"
                                                  " <transponder index>:<antenna index>.",
                                                  1,
                                                  std::numeric_limits<std::uint16_t>::max(),
                                                  "^\\d+:\\d+$");

  configRegistrar.registerNonNumeric<std::string>("transponder.receive.action",
                                                  ConfigurationProperties::REQUIRED,
                                                  {},
                                                  "Defines per transponder receive action:"
                                                  " 'ubend' or 'process' with the following format:"
                                                  " <transponder index>:ubend|process.",
                                                  1,
                                                  std::numeric_limits<std::uint16_t>::max(),
                                                  "^\\d+:(process|ubend)$");

  configRegistrar.registerNonNumeric<std::string>("transponder.receive.enable",
                                                  ConfigurationProperties::REQUIRED |
                                                  ConfigurationProperties::MODIFIABLE,
                                                  {},
                                                  "Defines per transponder receive enable:"
                                                  " 'yes' or 'on'; or 'no' or 'off' with the format:"
                                                  " <transponder index>:yes|on|no|off.",
                                                  1,
                                                  std::numeric_limits<std::uint16_t>::max(),
                                                  "^\\d+:(on|off|yes|no)$");

  configRegistrar.registerNonNumeric<std::string>("transponder.transmit.pcrcurveindex",
                                                  ConfigurationProperties::REQUIRED |
                                                  ConfigurationProperties::MODIFIABLE,
                                                  {},
                                                  "Defines per transponder transmit PCR curve index"
                                                  " with the following format:"
                                                  " <transponder index>:<pcrcurveindex>.",
                                                  1,
                                                  std::numeric_limits<std::uint16_t>::max(),
                                                  "^\\d+:\\d+$");

  configRegistrar.registerNonNumeric<std::string>("transponder.transmit.frequency",
                                                  ConfigurationProperties::REQUIRED |
                                                  ConfigurationProperties::MODIFIABLE,
                                                  {},
                                                  "Defines per transponder transmit center frequency in hz"
                                                  " with the following format: <transponder index>:<frequency hz>.",
                                                  1,
                                                  std::numeric_limits<std::uint16_t>::max(),
                                                  "^\\d+:\\d+(\\.\\d+){0,1}(G|M|K){0,1}$");

  configRegistrar.registerNonNumeric<std::string>("transponder.transmit.bandwidth",
                                                  ConfigurationProperties::REQUIRED,
                                                  {},
                                                  "Defines per transponder transmit bandwidth in hz"
                                                  " with the following format: <transponder index>:<frequency hz>.",
                                                  1,
                                                  std::numeric_limits<std::uint16_t>::max(),
                                                  "^\\d+:\\d+(\\.\\d+){0,1}(G|M|K){0,1}$");

  configRegistrar.registerNonNumeric<std::string>("transponder.transmit.antenna",
                                                  ConfigurationProperties::REQUIRED,
                                                  {},
                                                  "Defines per transponder transmit antenna"
                                                  " with the following format:"
                                                  " <transponder index>:<antenna index>.",
                                                  1,
                                                  std::numeric_limits<std::uint16_t>::max(),
                                                  "^\\d+:\\d+$");

  configRegistrar.registerNonNumeric<std::string>("transponder.transmit.ubend.delay",
                                                  ConfigurationProperties::REQUIRED |
                                                  ConfigurationProperties::MODIFIABLE,
                                                  {},
                                                  "Defines per transponder transmit u-bend delay in"
                                                  " microseconds. Applicable when associated"
                                                  " 'transponder.receive.action' is 'ubend' with the"
                                                  " following format: <transponder index>:<delay microseconds>.",
                                                  1,
                                                  std::numeric_limits<std::uint16_t>::max(),
                                                  "^\\d+:(na|\\d+)$");

  configRegistrar.registerNonNumeric<std::string>("transponder.transmit.datarate",
                                                  ConfigurationProperties::REQUIRED |
                                                  ConfigurationProperties::MODIFIABLE,
                                                  {},
                                                  "Defines per transponder transmit datarate in bps"
                                                  " with the following format: <transponder index>:<datarate bps>.",
                                                  1,
                                                  std::numeric_limits<std::uint16_t>::max(),
                                                  "^\\d+:\\d+(\\.\\d+){0,1}(G|M|K){0,1}$");

  configRegistrar.registerNonNumeric<std::string>("transponder.transmit.power",
                                                  ConfigurationProperties::REQUIRED |
                                                  ConfigurationProperties::MODIFIABLE,
                                                  {},
                                                  "Defines per transponder transmit power in dBm"
                                                  " with the following format: <transponder index>:<power dBm>.",
                                                  1,
                                                  std::numeric_limits<std::uint16_t>::max(),
                                                  "^\\d+:\\d+(\\.\\d+){0,1}(G|M|K){0,1}$");

  configRegistrar.registerNonNumeric<std::string>("transponder.transmit.tosmap",
                                                  ConfigurationProperties::REQUIRED,
                                                  {},
                                                  "Defines TOS or DSCP mapping to transponder index in order to"
                                                  " direct downstream frames to the appropriate transponder  when"
                                                  " operating in 'process' mode. Note: TOS or DSCP is dictated"
                                                  " by the boundary component in use. Specified with the following"
                                                  " format: <transponder index>:all|na|[(value|value-value)];....",
                                                  1,
                                                  std::numeric_limits<std::uint16_t>::max(),
                                                  "^\\d+:(na|((\\d+|\\d+-\\d+)(;){0,1}){1,})");

  configRegistrar.registerNonNumeric<std::string>("transponder.transmit.slotperframe",
                                                  ConfigurationProperties::REQUIRED |
                                                  ConfigurationProperties::MODIFIABLE,
                                                  {},
                                                  "Defines per transponder transmit slots per"
                                                  " frame with the following format:"
                                                  " <transponder index>:<slots per frame>.",
                                                  1,
                                                  std::numeric_limits<std::uint16_t>::max(),
                                                  "^\\d+:(na|\\d+)$");

  configRegistrar.registerNonNumeric<std::string>("transponder.transmit.slotsize",
                                                  ConfigurationProperties::REQUIRED |
                                                  ConfigurationProperties::MODIFIABLE,
                                                  {},
                                                  "Defines per transponder transmit slot size in"
                                                  " microseconds or 'na' with the"
                                                  " following format: <transponder index>:na|<slot size microseconds>.",
                                                  1,
                                                  std::numeric_limits<std::uint16_t>::max(),
                                                  "^\\d+:(na|\\d+)$");

  configRegistrar.registerNonNumeric<std::string>("transponder.transmit.txslots",
                                                  ConfigurationProperties::REQUIRED |
                                                  ConfigurationProperties::MODIFIABLE,
                                                  {},
                                                  "Defines per transponder transmit slots"
                                                  " with the following format:"
                                                  " <transponder index>:[slot|slot-slot];....",
                                                  1,
                                                  std::numeric_limits<std::uint16_t>::max(),
                                                  "^\\d+:(na|(\\d+|\\d+-\\d+)(;(\\d+|\\d+-\\d+)){0,})$");

  configRegistrar.registerNonNumeric<std::string>("transponder.transmit.mtu",
                                                  ConfigurationProperties::REQUIRED |
                                                  ConfigurationProperties::MODIFIABLE,
                                                  {},
                                                  "Defines per transponder transmit mtu in"
                                                  " bytes or 'na' if using transmit slots with "
                                                  " following format: <transponder index>:na|<slot size microseconds>.",
                                                  1,
                                                  std::numeric_limits<std::uint64_t>::max(),
                                                  "^\\d+:(na|\\d+)$");

  configRegistrar.registerNonNumeric<std::string>("transponder.transmit.enable",
                                                  ConfigurationProperties::REQUIRED |
                                                  ConfigurationProperties::MODIFIABLE,
                                                  {},
                                                  "Defines per transponder transmit enable:"
                                                  " 'yes' or 'on'; or 'no' or 'off' with the format:"
                                                  " <transponder index>:yes|on|no|off.",
                                                  1,
                                                  std::numeric_limits<std::uint16_t>::max(),
                                                  "^\\d+:(on|off|yes|no)$");

  configRegistrar.registerNonNumeric<std::string>("antenna.defines",
                                                  ConfigurationProperties::REQUIRED |
                                                  ConfigurationProperties::MODIFIABLE,
                                                  {},
                                                  "Defines antennas available for use by transponders"
                                                  " with the following format: <antenna index>:"
                                                  "(omni,<fixed gain dBi>)|(<antenna profile id,"
                                                  "azimuth degrees,elevation degress>),<spectrummask>.",
                                                  1,
                                                  std::numeric_limits<std::uint16_t>::max(),
                                                  "^\\d+:(omni;(-){0,1}\\d+(\\.\\d+){0,1}|\\d+;(-){0,1}\\d+(\\.\\d+){0,1};(-){0,1}\\d+(\\.\\d+){0,1});\\d+$");


  configRegistrar.registerNumeric<std::uint16_t>("reassembly.fragmentcheckthreshold",
                                                 ConfigurationProperties::DEFAULT,
                                                 {2},
                                                 "Defines the rate in seconds a check is performed to see if any packet"
                                                 " fragment reassembly efforts should be abandoned.");

  configRegistrar.registerNumeric<std::uint16_t>("reassembly.fragmenttimeoutthreshold",
                                                 ConfigurationProperties::DEFAULT,
                                                 {5},
                                                 "Defines the threshold in seconds to wait for another packet fragment"
                                                 " for an existing reassembly effort before abandoning the effort.");

  configRegistrar.registerNonNumeric<std::string>("pcrcurveuri",
                                                  ConfigurationProperties::REQUIRED,
                                                  {},
                                                  "Defines the URI of the Packet Completion Rate (PCR) curve"
                                                  " file. The PCR curve file contains probability of reception curves"
                                                  " as a function of Signal to Interference plus Noise Ratio (SINR).");



  configRegistrar.registerValidator(std::bind(&ConfigurationValidationManager::validate,
                                              &configurationValidationManager_,
                                              std::placeholders::_1));

  queueManager_.initialize(registrar);

  auto & statisticRegistrar = registrar.statisticRegistrar();

  packetStatusPublisher_.registerStatistics(statisticRegistrar);

  antennaStatusPublisher_.registerStatistics(statisticRegistrar);

  transponderStatusPublisher_.registerStatistics(statisticRegistrar);

  neighborStatusPublisher_.registerStatistics(statisticRegistrar);

  slotStatusPublisher_.registerStatistics(statisticRegistrar);
}


void
EMANE::Models::BentPipe::RadioModel::configure(const ConfigurationUpdate & update)
{
  LOGGER_STANDARD_LOGGING(pPlatformService_->logService(),
                          DEBUG_LEVEL,
                          "MACI %03hu BentPipe::RadioModel::%s",
                          id_,
                          __func__);

  loadConfiguration(update);
}


void
EMANE::Models::BentPipe::RadioModel::loadConfiguration(const ConfigurationUpdate & update)
{
  LOGGER_STANDARD_LOGGING(pPlatformService_->logService(),
                          DEBUG_LEVEL,
                          "MACI %03hu BentPipe::RadioModel::%s",
                          id_,
                          __func__);

  TransponderConfigurations transponderConfigurations{};
  ConfigurationUpdate queueManagerConfigurationUpdate{};

  std::chrono::seconds fragmentCheckThreshold{};
  std::chrono::seconds fragmentTimeoutThreshold{};

  for(const auto & item : update)
    {
      if(item.first == "transponder.receive.frequency")
        {
          for(const auto & entry : item.second)
            {
              configureTransponderValue(transponderConfigurations,
                                        entry.asString(),
                                        &TransponderConfiguration::setReceiveFrequencyHz);

              LOGGER_STANDARD_LOGGING(pPlatformService_->logService(),
                                      INFO_LEVEL,
                                      "MACI %03hu BentPipe::RadioModel::%s %s = %s",
                                      id_,
                                      __func__,
                                      item.first.c_str(),
                                      entry.asString().c_str());
            }
        }
      else if(item.first == "transponder.receive.bandwidth")
        {
          for(const auto & entry : item.second)
            {
              configureTransponderValue(transponderConfigurations,
                                        entry.asString(),
                                        &TransponderConfiguration::setReceiveBandwidthHz);

              LOGGER_STANDARD_LOGGING(pPlatformService_->logService(),
                                      INFO_LEVEL,
                                      "MACI %03hu BentPipe::RadioModel::%s %s = %s",
                                      id_,
                                      __func__,
                                      item.first.c_str(),
                                      entry.asString().c_str());
            }
        }
      else if(item.first == "transponder.receive.action")
        {
          for(const auto & entry : item.second)
            {
              configureTransponderValue(transponderConfigurations,
                                        entry.asString(),
                                        &TransponderConfiguration::setReceiveAction);

              LOGGER_STANDARD_LOGGING(pPlatformService_->logService(),
                                      INFO_LEVEL,
                                      "MACI %03hu BentPipe::RadioModel::%s %s = %s",
                                      id_,
                                      __func__,
                                      item.first.c_str(),
                                      entry.asString().c_str());
            }
        }
      else if(item.first == "transponder.receive.antenna")
        {
          for(const auto & entry : item.second)
            {
              configureTransponderValue(transponderConfigurations,
                                        entry.asString(),
                                        &TransponderConfiguration::setReceiveAntennaIndex);

              LOGGER_STANDARD_LOGGING(pPlatformService_->logService(),
                                      INFO_LEVEL,
                                      "MACI %03hu BentPipe::RadioModel::%s %s = %s",
                                      id_,
                                      __func__,
                                      item.first.c_str(),
                                      entry.asString().c_str());
            }
        }
      else if(item.first == "transponder.receive.enable")
        {
          for(const auto & entry : item.second)
            {
              configureTransponderValue(transponderConfigurations,
                                        entry.asString(),
                                        &TransponderConfiguration::setReceiveEnable);

              LOGGER_STANDARD_LOGGING(pPlatformService_->logService(),
                                      INFO_LEVEL,
                                      "MACI %03hu BentPipe::RadioModel::%s %s = %s",
                                      id_,
                                      __func__,
                                      item.first.c_str(),
                                      entry.asString().c_str());
            }
        }
      else if(item.first == "transponder.transmit.pcrcurveindex")
        {
          for(const auto & entry : item.second)
            {
              configureTransponderValue(transponderConfigurations,
                                        entry.asString(),
                                        &TransponderConfiguration::setPCRCurveIndex);

              LOGGER_STANDARD_LOGGING(pPlatformService_->logService(),
                                      INFO_LEVEL,
                                      "MACI %03hu BentPipe::RadioModel::%s %s = %s",
                                      id_,
                                      __func__,
                                      item.first.c_str(),
                                      entry.asString().c_str());
            }
        }
      else if(item.first == "transponder.transmit.frequency")
        {
          for(const auto & entry : item.second)
            {
              configureTransponderValue(transponderConfigurations,
                                        entry.asString(),
                                        &TransponderConfiguration::setTransmitFrequencyHz);

              LOGGER_STANDARD_LOGGING(pPlatformService_->logService(),
                                      INFO_LEVEL,
                                      "MACI %03hu BentPipe::RadioModel::%s %s = %s",
                                      id_,
                                      __func__,
                                      item.first.c_str(),
                                      entry.asString().c_str());
            }
        }
      else if(item.first == "transponder.transmit.bandwidth")
        {
          for(const auto & entry : item.second)
            {
              configureTransponderValue(transponderConfigurations,
                                        entry.asString(),
                                        &TransponderConfiguration::setTransmitBandwidthHz);

              LOGGER_STANDARD_LOGGING(pPlatformService_->logService(),
                                      INFO_LEVEL,
                                      "MACI %03hu BentPipe::RadioModel::%s %s = %s",
                                      id_,
                                      __func__,
                                      item.first.c_str(),
                                      entry.asString().c_str());
            }
        }
      else if(item.first == "transponder.transmit.ubend.delay")
        {
          for(const auto & entry : item.second)
            {
              configureTransponderValue(transponderConfigurations,
                                        entry.asString(),
                                        &TransponderConfiguration::setTransmitUbendDelay);

              LOGGER_STANDARD_LOGGING(pPlatformService_->logService(),
                                      INFO_LEVEL,
                                      "MACI %03hu BentPipe::RadioModel::%s %s = %s",
                                      id_,
                                      __func__,
                                      item.first.c_str(),
                                      entry.asString().c_str());
            }
        }
      else if(item.first == "transponder.transmit.datarate")
        {
          for(const auto & entry : item.second)
            {
              configureTransponderValue(transponderConfigurations,
                                        entry.asString(),
                                        &TransponderConfiguration::setTransmitDataRatebps);

              LOGGER_STANDARD_LOGGING(pPlatformService_->logService(),
                                      INFO_LEVEL,
                                      "MACI %03hu BentPipe::RadioModel::%s %s = %s",
                                      id_,
                                      __func__,
                                      item.first.c_str(),
                                      entry.asString().c_str());
            }
        }
      else if(item.first == "transponder.transmit.tosmap")
        {
          for(const auto & entry : item.second)
            {
              configureTransponderValue(transponderConfigurations,
                                        entry.asString(),
                                        &TransponderConfiguration::setTransmitProcessTOS);

              LOGGER_STANDARD_LOGGING(pPlatformService_->logService(),
                                      INFO_LEVEL,
                                      "MACI %03hu BentPipe::RadioModel::%s %s = %s",
                                      id_,
                                      __func__,
                                      item.first.c_str(),
                                      entry.asString().c_str());
            }
        }
      else if(item.first == "transponder.transmit.slotperframe")
        {
          for(const auto & entry : item.second)
            {
              configureTransponderValue(transponderConfigurations,
                                        entry.asString(),
                                        &TransponderConfiguration::setTransmitSlotsPerFrame);

              LOGGER_STANDARD_LOGGING(pPlatformService_->logService(),
                                      INFO_LEVEL,
                                      "MACI %03hu BentPipe::RadioModel::%s %s = %s",
                                      id_,
                                      __func__,
                                      item.first.c_str(),
                                      entry.asString().c_str());
            }
        }
      else if(item.first == "transponder.transmit.slotsize")
        {
          for(const auto & entry : item.second)
            {
              configureTransponderValue(transponderConfigurations,
                                        entry.asString(),
                                        &TransponderConfiguration::setTransmitSlotSize);

              LOGGER_STANDARD_LOGGING(pPlatformService_->logService(),
                                      INFO_LEVEL,
                                      "MACI %03hu BentPipe::RadioModel::%s %s = %s",
                                      id_,
                                      __func__,
                                      item.first.c_str(),
                                      entry.asString().c_str());
            }
        }
      else if(item.first == "transponder.transmit.txslots")
        {
          for(const auto & entry : item.second)
            {
              configureTransponderValue(transponderConfigurations,
                                        entry.asString(),
                                        &TransponderConfiguration::setTransmitSlots);

              LOGGER_STANDARD_LOGGING(pPlatformService_->logService(),
                                      INFO_LEVEL,
                                      "MACI %03hu BentPipe::RadioModel::%s %s = %s",
                                      id_,
                                      __func__,
                                      item.first.c_str(),
                                      entry.asString().c_str());
            }
        }
      else if(item.first == "transponder.transmit.mtu")
        {
          for(const auto & entry : item.second)
            {
              configureTransponderValue(transponderConfigurations,
                                        entry.asString(),
                                        &TransponderConfiguration::setTransmitMTUBytes);

              LOGGER_STANDARD_LOGGING(pPlatformService_->logService(),
                                      INFO_LEVEL,
                                      "MACI %03hu BentPipe::RadioModel::%s %s = %s",
                                      id_,
                                      __func__,
                                      item.first.c_str(),
                                      entry.asString().c_str());
            }
        }
      else if(item.first == "transponder.transmit.antenna")
        {
          for(const auto & entry : item.second)
            {
              configureTransponderValue(transponderConfigurations,
                                        entry.asString(),
                                        &TransponderConfiguration::setTransmitAntennaIndex);

              LOGGER_STANDARD_LOGGING(pPlatformService_->logService(),
                                      INFO_LEVEL,
                                      "MACI %03hu BentPipe::RadioModel::%s %s = %s",
                                      id_,
                                      __func__,
                                      item.first.c_str(),
                                      entry.asString().c_str());
            }
        }
      else if(item.first == "transponder.transmit.power")
        {
          for(const auto & entry : item.second)
            {
              configureTransponderValue(transponderConfigurations,
                                        entry.asString(),
                                        &TransponderConfiguration::setTransmitPowerdBm);

              LOGGER_STANDARD_LOGGING(pPlatformService_->logService(),
                                      INFO_LEVEL,
                                      "MACI %03hu BentPipe::RadioModel::%s %s = %s",
                                      id_,
                                      __func__,
                                      item.first.c_str(),
                                      entry.asString().c_str());
            }
        }
      else if(item.first == "transponder.transmit.enable")
        {
          for(const auto & entry : item.second)
            {
              configureTransponderValue(transponderConfigurations,
                                        entry.asString(),
                                        &TransponderConfiguration::setTransmitEnable);

              LOGGER_STANDARD_LOGGING(pPlatformService_->logService(),
                                      INFO_LEVEL,
                                      "MACI %03hu BentPipe::RadioModel::%s %s = %s",
                                      id_,
                                      __func__,
                                      item.first.c_str(),
                                      entry.asString().c_str());
            }
        }
      else if(item.first == "antenna.defines")
        {
          for(const auto & entry : item.second)
            {
              configureAntennaValue(antennas_,
                                    entry.asString());

              LOGGER_STANDARD_LOGGING(pPlatformService_->logService(),
                                      INFO_LEVEL,
                                      "MACI %03hu BentPipe::RadioModel::%s %s = %s",
                                      id_,
                                      __func__,
                                      item.first.c_str(),
                                      entry.asString().c_str());
            }
        }
      else if(item.first == "reassembly.fragmentcheckthreshold")
        {
          fragmentCheckThreshold = std::chrono::seconds{item.second[0].asUINT16()};

          LOGGER_STANDARD_LOGGING(pPlatformService_->logService(),
                                  INFO_LEVEL,
                                  "MACI %03hu BentPipe::RadioModel::%s: %s = %lu",
                                  id_,
                                  __func__,
                                  item.first.c_str(),
                                  fragmentCheckThreshold.count());
        }
      else if(item.first == "reassembly.fragmenttimeoutthreshold")
        {
          fragmentTimeoutThreshold = std::chrono::seconds{item.second[0].asUINT16()};

          LOGGER_STANDARD_LOGGING(pPlatformService_->logService(),
                                  INFO_LEVEL,
                                  "MACI %03hu BentPipe::RadioModel::%s: %s = %lu",
                                  id_,
                                  __func__,
                                  item.first.c_str(),
                                  fragmentTimeoutThreshold.count());
        }
      else if(item.first == "pcrcurveuri")
        {
          std::string sPCRCurveURI = item.second[0].asString();

          LOGGER_STANDARD_LOGGING(pPlatformService_->logService(),
                                  INFO_LEVEL,
                                  "MACI %03hu BentPipe::RadioModel::%s: %s = %s",
                                  id_,
                                  __func__,
                                  item.first.c_str(),
                                  sPCRCurveURI.c_str());

          pcrManager_.load(sPCRCurveURI);
        }
      else
        {

          if(!item.first.compare(0,
                                 strlen(QueueManager::CONFIG_PREFIX),
                                 QueueManager::CONFIG_PREFIX))
            {
              queueManagerConfigurationUpdate.push_back(item);
            }
          else
            {
              throw makeException<ConfigureException>("BentPipe::RadioModel: "
                                                      "Unexpected configuration item %s.",
                                                      item.first.c_str());
            }
        }
    }

  queueManager_.configure(queueManagerConfigurationUpdate);

  AntennaIndexSet rxAntennaIndexes{};

  for(const auto & entry : transponderConfigurations)
    {
      // set the rx antenna bandwidth and verify that all transponders
      // sharing an rx antenna are using the same rx bandwidth

      TransponderIndex transponderIndex = entry.first;
      auto & transponderConfiguration = entry.second;

      AntennaIndex antennaIndex = transponderConfiguration.getReceiveAntennaIndex();

      auto antennaFrequencyMapIter = antennaFrequencyMap_.find(antennaIndex);

      if(antennaFrequencyMapIter == antennaFrequencyMap_.end())
        {
          antennaFrequencyMapIter =
            antennaFrequencyMap_.emplace(antennaIndex,FrequencySet{}).first;
        }

      // keep track of the rx frequencies of interest for this antenna
      antennaFrequencyMapIter->second.emplace(transponderConfiguration.getReceiveFrequencyHz());

      auto key = std::make_pair(antennaIndex,
                                transponderConfiguration.getReceiveFrequencyHz());

      auto iter = antennaTransponderMap_.find(key);

      if(iter == antennaTransponderMap_.end())
        {
          iter = antennaTransponderMap_.emplace(key,transponderIndex).first;

          LOGGER_VERBOSE_LOGGING(pPlatformService_->logService(),
                                 DEBUG_LEVEL,
                                 "transponder %hu adding antenna %hu @ rx %zu",
                                 transponderIndex,
                                 antennaIndex,
                                 transponderConfiguration.getReceiveFrequencyHz());


        }
      else
        {
          makeException<ConfigureException>("BentPipe::RadioModel: Transponder %hu attempting"
                                            " to use antenna index %hu and receive frequency"
                                            " %zu already in use",
                                            transponderIndex,
                                            transponderConfiguration.getReceiveFrequencyHz(),
                                            antennaIndex);
        }

      if(!rxAntennaIndexes.count(antennaIndex))
        {
          antennas_[antennaIndex].setBandwidthHz(transponderConfiguration.getReceiveBandwidthHz());

          rxAntennaIndexes.emplace(antennaIndex);
        }
      else
        {
          if(antennas_[antennaIndex].getBandwidthHz() !=
             entry.second.getReceiveBandwidthHz())
            {
              throw makeException<ConfigureException>("BentPipe::RadioModel: Multiple"
                                                      " transponders with different"
                                                      " receive bandwidth cannot use"
                                                      " the same antenna index %hu",
                                                      antennaIndex);
            }
        }
      Transponder * pTransponder{};

      if(transponderConfiguration.getTransmitSlotSize().count())
        {
          // create a TDMA protocol transponder
          pTransponder = new TransponderTDMAProtocol{id_,
                                                     pPlatformService_,
                                                     this,
                                                     transponderConfiguration,
                                                     &slotStatusPublisher_};
        }
      else
        {
          // create a no protocol transponder
          pTransponder = new TransponderNoProtocol{id_,
                                                   pPlatformService_,
                                                   this,
                                                   transponderConfiguration};
        }

      transponderStatusPublisher_.addTransponder(*pTransponder);

      transponders_.emplace(transponderIndex,pTransponder);

      // create a per transponder receive manager
      receiveManagers_.emplace(transponderIndex,
                               new ReceiveManager{id_,
                                                  transponderIndex,
                                                  this,
                                                  pPlatformService_,
                                                  pRadioService_,
                                                  &packetStatusPublisher_,
                                                  &neighborStatusPublisher_,
                                                  transponderConfiguration.getReceiveAction() ==
                                                  ReceiveAction::PROCESS,
                                                  &pcrManager_,
                                                  transponderConfiguration.getReceiveAntennaIndex(),
                                                  fragmentCheckThreshold,
                                                  fragmentTimeoutThreshold});

      queueManager_.addQueue(transponderIndex);

      for(auto i :  entry.second.getTransmitProcessTOS())
        {
          if(tosToTransponder_[i] == -1)
            {
              tosToTransponder_[i] = transponderIndex;
            }
          else
            {
              throw makeException<ConfigureException>("BentPipe::RadioModel: Multiple"
                                                      " transponders (%hu and %hu) mapped to TOS"
                                                      " %hhu ",
                                                      tosToTransponder_[i],
                                                      transponderIndex,
                                                      i);
            }
        }
    }
}

void
EMANE::Models::BentPipe::RadioModel::start()
{}


void
EMANE::Models::BentPipe::RadioModel::postStart()
{
  LOGGER_STANDARD_LOGGING(pPlatformService_->logService(),
                          INFO_LEVEL,
                          "MACI %03hu BendPipe::RadioModel::%s",
                          id_,
                          __func__);

  for(const auto & entry : antennaFrequencyMap_)
    {
      auto rxAntennaIndex = entry.first;
      const auto & rxAntennaFrequencySet = entry.second;

      if(auto iterAntenna = antennas_.find(rxAntennaIndex);
         iterAntenna !=  antennas_.end())
        {
          sendDownstreamControl({Controls::RxAntennaAddControlMessage::create(iterAntenna->second,
                                                                              rxAntennaFrequencySet)});

          antennaStatusPublisher_.addAntenna(iterAntenna->second,
                                             rxAntennaFrequencySet);

          registeredRxAntenna_.emplace(rxAntennaIndex);
        }
      else
        {
          LOGGER_STANDARD_LOGGING(pPlatformService_->logService(),
                                  ERROR_LEVEL,
                                  "MACI %03hu BendPipe::RadioModel::%s"
                                  " attempt to use unknown antenna index:%hu",
                                  id_,
                                  __func__,
                                  rxAntennaIndex);
        }
    }

  for(auto & entry : transponders_)
    {
      entry.second->start();
    }
}


void
EMANE::Models::BentPipe::RadioModel::stop()
{
  for(auto & entry : transponders_)
    {
      entry.second->stop();
    }

  for(const auto & antennaIndex : registeredRxAntenna_)
    {
      sendDownstreamControl({Controls::RxAntennaRemoveControlMessage::create(antennaIndex)});
    }

  registeredRxAntenna_.clear();
}


void
EMANE::Models::BentPipe::RadioModel::destroy() noexcept
{}


void EMANE::Models::BentPipe::RadioModel::processUpstreamControl(const ControlMessages &)
{}


void EMANE::Models::BentPipe::RadioModel::processUpstreamPacket(const CommonMACHeader & hdr, UpstreamPacket & pkt, const ControlMessages & msgs)
{
  emane_bentpipe_radiomodel_processUpstreamPacket(this, &hdr, &pkt, &msgs);
}


void EMANE::Models::BentPipe::RadioModel::processDownstreamControl(const ControlMessages &)
{}


void EMANE::Models::BentPipe::RadioModel::processDownstreamPacket(DownstreamPacket & pkt, const ControlMessages & msgs)
{
  emane_bentpipe_radiomodel_processDownstreamPacket(this, &pkt, &msgs);
}


void EMANE::Models::BentPipe::RadioModel::processConfiguration(const ConfigurationUpdate & update)
{
  LOGGER_STANDARD_LOGGING(pPlatformService_->logService(),
                          DEBUG_LEVEL,
                          "MACI %03hu BentPipe::RadioModel::%s",
                          id_,
                          __func__);

  TransponderConfigurationUpdates configurationUpdates{};
  Antennas antennaUpdates{};

  AntennaIndexSet modifiedAntennaSet{};

  for(const auto & item : update)
    {
      if(item.first == "transponder.receive.frequency")
        {
          for(const auto & entry : item.second)
            {
              configureTransponderValue(configurationUpdates,
                                        entry.asString(),
                                        &TransponderConfigurationUpdate::setReceiveFrequencyHz);

              LOGGER_STANDARD_LOGGING(pPlatformService_->logService(),
                                      INFO_LEVEL,
                                      "MACI %03hu BentPipe::RadioModel::%s %s = %s",
                                      id_,
                                      __func__,
                                      item.first.c_str(),
                                      entry.asString().c_str());
            }
        }
      else if(item.first == "transponder.receive.enable")
        {
          for(const auto & entry : item.second)
            {
              configureTransponderValue(configurationUpdates,
                                        entry.asString(),
                                        &TransponderConfigurationUpdate::setReceiveEnable);

              LOGGER_STANDARD_LOGGING(pPlatformService_->logService(),
                                      INFO_LEVEL,
                                      "MACI %03hu BentPipe::RadioModel::%s %s = %s",
                                      id_,
                                      __func__,
                                      item.first.c_str(),
                                      entry.asString().c_str());
            }
        }
      else if(item.first == "transponder.transmit.pcrcurveindex")
        {
          for(const auto & entry : item.second)
            {
              configureTransponderValue(configurationUpdates,
                                        entry.asString(),
                                        &TransponderConfigurationUpdate::setPCRCurveIndex);

              LOGGER_STANDARD_LOGGING(pPlatformService_->logService(),
                                      INFO_LEVEL,
                                      "MACI %03hu BentPipe::RadioModel::%s %s = %s",
                                      id_,
                                      __func__,
                                      item.first.c_str(),
                                      entry.asString().c_str());
            }
        }
      else if(item.first == "transponder.transmit.frequency")
        {
          for(const auto & entry : item.second)
            {
              configureTransponderValue(configurationUpdates,
                                        entry.asString(),
                                        &TransponderConfigurationUpdate::setTransmitFrequencyHz);

              LOGGER_STANDARD_LOGGING(pPlatformService_->logService(),
                                      INFO_LEVEL,
                                      "MACI %03hu BentPipe::RadioModel::%s %s = %s",
                                      id_,
                                      __func__,
                                      item.first.c_str(),
                                      entry.asString().c_str());
            }
        }
      else if(item.first == "transponder.transmit.ubend.delay")
        {
          for(const auto & entry : item.second)
            {
              configureTransponderValue(configurationUpdates,
                                        entry.asString(),
                                        &TransponderConfigurationUpdate::setTransmitUbendDelay);

              LOGGER_STANDARD_LOGGING(pPlatformService_->logService(),
                                      INFO_LEVEL,
                                      "MACI %03hu BentPipe::RadioModel::%s %s = %s",
                                      id_,
                                      __func__,
                                      item.first.c_str(),
                                      entry.asString().c_str());
            }
        }
      else if(item.first == "transponder.transmit.datarate")
        {
          for(const auto & entry : item.second)
            {
              configureTransponderValue(configurationUpdates,
                                        entry.asString(),
                                        &TransponderConfigurationUpdate::setTransmitDataRatebps);

              LOGGER_STANDARD_LOGGING(pPlatformService_->logService(),
                                      INFO_LEVEL,
                                      "MACI %03hu BentPipe::RadioModel::%s %s = %s",
                                      id_,
                                      __func__,
                                      item.first.c_str(),
                                      entry.asString().c_str());
            }
        }
      else if(item.first == "transponder.transmit.mtu")
        {
          for(const auto & entry : item.second)
            {
              configureTransponderValue(configurationUpdates,
                                        entry.asString(),
                                        &TransponderConfigurationUpdate::setTransmitMTUBytes);

              LOGGER_STANDARD_LOGGING(pPlatformService_->logService(),
                                      INFO_LEVEL,
                                      "MACI %03hu BentPipe::RadioModel::%s %s = %s",
                                      id_,
                                      __func__,
                                      item.first.c_str(),
                                      entry.asString().c_str());
            }
        }
      else if(item.first == "transponder.transmit.slotperframe")
        {
          for(const auto & entry : item.second)
            {
              configureTransponderValue(configurationUpdates,
                                        entry.asString(),
                                        &TransponderConfigurationUpdate::setTransmitSlotsPerFrame);

              LOGGER_STANDARD_LOGGING(pPlatformService_->logService(),
                                      INFO_LEVEL,
                                      "MACI %03hu BentPipe::RadioModel::%s %s = %s",
                                      id_,
                                      __func__,
                                      item.first.c_str(),
                                      entry.asString().c_str());
            }
        }
      else if(item.first == "transponder.transmit.slotsize")
        {
          for(const auto & entry : item.second)
            {
              configureTransponderValue(configurationUpdates,
                                        entry.asString(),
                                        &TransponderConfigurationUpdate::setTransmitSlotSize);

              LOGGER_STANDARD_LOGGING(pPlatformService_->logService(),
                                      INFO_LEVEL,
                                      "MACI %03hu BentPipe::RadioModel::%s %s = %s",
                                      id_,
                                      __func__,
                                      item.first.c_str(),
                                      entry.asString().c_str());
            }
        }
      else if(item.first == "transponder.transmit.txslots")
        {
          for(const auto & entry : item.second)
            {
              configureTransponderValue(configurationUpdates,
                                        entry.asString(),
                                        &TransponderConfigurationUpdate::setTransmitSlots);

              LOGGER_STANDARD_LOGGING(pPlatformService_->logService(),
                                      INFO_LEVEL,
                                      "MACI %03hu BentPipe::RadioModel::%s %s = %s",
                                      id_,
                                      __func__,
                                      item.first.c_str(),
                                      entry.asString().c_str());
            }
        }
      else if(item.first == "transponder.transmit.power")
        {
          for(const auto & entry : item.second)
            {
              configureTransponderValue(configurationUpdates,
                                        entry.asString(),
                                        &TransponderConfigurationUpdate::setTransmitPowerdBm);

              LOGGER_STANDARD_LOGGING(pPlatformService_->logService(),
                                      INFO_LEVEL,
                                      "MACI %03hu BentPipe::RadioModel::%s %s = %s",
                                      id_,
                                      __func__,
                                      item.first.c_str(),
                                      entry.asString().c_str());
            }
        }
      else if(item.first == "transponder.transmit.enable")
        {
          for(const auto & entry : item.second)
            {
              configureTransponderValue(configurationUpdates,
                                        entry.asString(),
                                        &TransponderConfigurationUpdate::setTransmitEnable);

              LOGGER_STANDARD_LOGGING(pPlatformService_->logService(),
                                      INFO_LEVEL,
                                      "MACI %03hu BentPipe::RadioModel::%s %s = %s",
                                      id_,
                                      __func__,
                                      item.first.c_str(),
                                      entry.asString().c_str());
            }
        }
      else if(item.first == "antenna.defines")
        {
          for(const auto & entry : item.second)
            {
              configureAntennaValue(antennaUpdates,
                                    entry.asString());

              LOGGER_STANDARD_LOGGING(pPlatformService_->logService(),
                                      INFO_LEVEL,
                                      "MACI %03hu BentPipe::RadioModel::%s %s = %s",
                                      id_,
                                      __func__,
                                      item.first.c_str(),
                                      entry.asString().c_str());
            }
        }
    }

  for(const auto & entry : configurationUpdates)
    {
      // set the rx antenna bandwidth and verify that all transponders
      // sharing an rx antenna are using the same rx bandwidth

      TransponderIndex transponderIndex = entry.first;

      const auto & configurationUpdate = entry.second;

      auto & currentConfiguration = transponders_[transponderIndex]->getConfiguration();

      if(const auto & frequencyHz = configurationUpdate.getReceiveFrequencyHz())
        {
          std::uint64_t u64CurrentFrequencyHz{currentConfiguration.getReceiveFrequencyHz()};

          if(currentConfiguration.getReceiveFrequencyHz() != *frequencyHz)
            {
              currentConfiguration.setReceiveFrequencyHz(*frequencyHz);

              AntennaIndex rxAntennaIndex{currentConfiguration.getReceiveAntennaIndex()};

              if(auto iter = antennaFrequencyMap_.find(rxAntennaIndex);
                 iter != antennaFrequencyMap_.end())
                {
                  iter->second.erase(u64CurrentFrequencyHz);

                  iter->second.emplace(*frequencyHz);

                  modifiedAntennaSet.emplace(rxAntennaIndex);
                }

              // remove the antenna transponder mapping
              auto key = std::make_pair(rxAntennaIndex,
                                        u64CurrentFrequencyHz);

              antennaTransponderMap_.erase(key);

              // add the new antenna transponder mapping
              key = std::make_pair(rxAntennaIndex,*frequencyHz);

              auto iter = antennaTransponderMap_.find(key);

              if(iter == antennaTransponderMap_.end())
                {
                  iter = antennaTransponderMap_.emplace(key,transponderIndex).first;

                  LOGGER_VERBOSE_LOGGING(pPlatformService_->logService(),
                                         DEBUG_LEVEL,
                                         "MACI %03hu BentPipe::RadioModel::%s transponder"
                                         " %hu adding antenna %hu @ rx %zu",
                                         id_,
                                         __func__,
                                         transponderIndex,
                                         rxAntennaIndex,
                                         *frequencyHz);
                }
              else
                {
                  LOGGER_VERBOSE_LOGGING(pPlatformService_->logService(),
                                         ERROR_LEVEL,
                                         "MACI %03hu BentPipe::RadioModel::%s Transponder %hu attempting"
                                         " to use antenna index %hu and receive frequency"
                                         " %zu already in use",
                                         id_,
                                         __func__,
                                         transponderIndex,
                                         rxAntennaIndex,
                                         *frequencyHz);
                }
            }
        }

      if(const auto & index = configurationUpdate.getPCRCurveIndex())
        {
          if(currentConfiguration.getPCRCurveIndex() != *index)
            {
              currentConfiguration.setPCRCurveIndex(*index);
            }
        }

      if(const auto & frequencyHz  = configurationUpdate.getTransmitFrequencyHz())
        {
          if(currentConfiguration.getTransmitFrequencyHz() != *frequencyHz)
            {
              currentConfiguration.setTransmitFrequencyHz(*frequencyHz);
            }
        }

      if(const auto & dataratebps = configurationUpdate.getTransmitDataRatebps())
        {
          if(currentConfiguration.getTransmitDataRatebps() != *dataratebps)
            {
              currentConfiguration.setTransmitDataRatebps(*dataratebps);
            }
        }

      if(const auto & mtuBytes = configurationUpdate.getTransmitMTUBytes())
        {
          if(currentConfiguration.getTransmitMTUBytes() != *mtuBytes)
            {
              currentConfiguration.setTransmitMTUBytes(*mtuBytes);
            }
        }

      if(const auto & powerdBm = configurationUpdate.getTransmitPowerdBm())
        {
          if(currentConfiguration.getTransmitPowerdBm() != *powerdBm)
            {
              currentConfiguration.setTransmitPowerdBm(*powerdBm);
            }
        }

      if(const auto & delayMicroseconds = configurationUpdate.getTransmitUbendDelay())
        {
          if(currentConfiguration.getTransmitUbendDelay() != *delayMicroseconds)
            {
              currentConfiguration.setTransmitUbendDelay(*delayMicroseconds);
            }
        }

      if(const auto & slotsPerFrame  = configurationUpdate.getTransmitSlotsPerFrame())
        {
          if(currentConfiguration.getTransmitSlotsPerFrame() != *slotsPerFrame)
            {
              currentConfiguration.setTransmitSlotsPerFrame(*slotsPerFrame);
            }
        }

      if(const auto & slotSizeMicroseconds = configurationUpdate.getTransmitSlotSize())
        {
          if(currentConfiguration.getTransmitSlotSize() != *slotSizeMicroseconds)
            {
              currentConfiguration.setTransmitSlotSize(*slotSizeMicroseconds);
            }
        }

      if(const auto & slots = configurationUpdate.getTransmitSlots())
        {
          if(currentConfiguration.getTransmitSlots() != *slots)
            {
              currentConfiguration.setTransmitSlots(*slots);
            }
        }

      if(const auto & index = configurationUpdate.getReceiveEnable())
        {
          if(currentConfiguration.getReceiveEnable() != *index)
            {
              currentConfiguration.setReceiveEnable(*index);
            }
        }

      if(const auto & index = configurationUpdate.getTransmitEnable())
        {
          if(currentConfiguration.getTransmitEnable() != *index)
            {
              currentConfiguration.setTransmitEnable(*index);
            }
        }

      // update transponder status table
      transponderStatusPublisher_.removeTransponder(*transponders_[transponderIndex]);
      transponderStatusPublisher_.addTransponder(*transponders_[transponderIndex]);
    }

  // if antennas have been updated
  for(auto & entry : antennaUpdates)
    {
      AntennaIndex antennaIndex = entry.first;
      Antenna & antennaUpdate = entry.second;

      // find what we know about the current antenna with this index
      if(auto iterAntenna = antennas_.find(antennaIndex);
         iterAntenna != antennas_.end())
        {
          // set the updated antenna bandwidth, this is the same as the
          // original configured value
          antennaUpdate.setBandwidthHz(iterAntenna->second.getBandwidthHz());

          // we only need to notify the emulator physical layer if
          // this is an rx antenna, detectable if we have a frequency
          // set for it
          if(auto iterFrequencyMap = antennaFrequencyMap_.find(antennaIndex);
             iterFrequencyMap !=  antennaFrequencyMap_.end())
            {
              const auto & rxAntennaFrequencySet = iterFrequencyMap->second;

              if(antennaUpdate.isProfileDefined() &&
                 iterAntenna->second.isProfileDefined())
                {
                  // this is just an update
                  sendDownstreamControl({Controls::RxAntennaUpdateControlMessage::create(antennaUpdate)});
                }
              else
                {
                  // changing from/to an omni/profile defined -- we
                  // can do that the hard way
                  sendDownstreamControl({Controls::RxAntennaRemoveControlMessage::create(antennaIndex),
                      Controls::RxAntennaAddControlMessage::create(antennaUpdate,
                                                                   rxAntennaFrequencySet)});
                }

              antennaStatusPublisher_.removeAntenna(iterAntenna->second);

              antennaStatusPublisher_.addAntenna(antennaUpdate,
                                                 rxAntennaFrequencySet);
            }

          // update antenna configuration known to the radio model store
          iterAntenna->second = antennaUpdate;
        }
    }

  // if receive frequency has changed, we need to reinitialize the
  // modified antennas -- antenna spectrum configuration (rx
  // frequencies and bandwidth) correlate to individual spectrum
  // monitors internal to the emulator physical layer.
  for(auto rxAntennaIndex : modifiedAntennaSet)
    {
      if(auto iterAntenna = antennas_.find(rxAntennaIndex);
         iterAntenna !=  antennas_.end())
        {
          if(auto iterFrequencyMap = antennaFrequencyMap_.find(rxAntennaIndex);
             iterFrequencyMap !=  antennaFrequencyMap_.end())
            {
              const auto & rxAntennaFrequencySet = iterFrequencyMap->second;

              sendDownstreamControl({Controls::RxAntennaRemoveControlMessage::create(rxAntennaIndex),
                  Controls::RxAntennaAddControlMessage::create(iterAntenna->second,
                                                               rxAntennaFrequencySet)});

              antennaStatusPublisher_.removeAntenna(iterAntenna->second);

              antennaStatusPublisher_.addAntenna(iterAntenna->second,
                                                 rxAntennaFrequencySet);
            }
        }
    }
}


void EMANE::Models::BentPipe::RadioModel::process(const TimePoint & now)
{
  // check to see if any of the transponders have a tx opportunity
  for(auto & entry : transponders_)
    {
      auto & pTransponder{entry.second};

      if(pTransponder->isTransmitOpportunity(now))
        {
          auto entry = queueManager_.dequeue(pTransponder->getIndex(),
                                             pTransponder->getMTUBytes());

          MessageComponents & components = std::get<0>(entry);

          size_t totalSizeBytes{std::get<1>(entry)};

          if(totalSizeBytes)
            {
              const auto & configuration = pTransponder->getConfiguration();

              auto now = Clock::now();

              NEMId dst{};

              // determine whether to use a unicast or broadcast nem address
              for(const auto & component : components)
                {
                  // if not set, set a destination
                  if(!dst)
                    {
                      dst = component.getDestination();
                    }
                  else if(dst != NEM_BROADCAST_MAC_ADDRESS)
                    {
                      // if the destination is not broadcast, check to see if it matches
                      // the destination of the current component - if not, set the NEM
                      // broadcast address as the dst
                      if(dst != component.getDestination())
                        {
                          dst = NEM_BROADCAST_MAC_ADDRESS;
                          break;
                        }
                    }
                }

              Transponder::TransmissionInfo transmissionInfo{pTransponder->prepareTransmission(now,
                                                                                               totalSizeBytes,
                                                                                               std::move(components))};

              BentPipeMessage & bentPipeMessage = std::get<0>(transmissionInfo);

              if(configuration.getTransmitEnable())
                {
                  Microseconds & durationMicroseconds = std::get<1>(transmissionInfo);

                  // there is something to send
                  Serialization serialization{bentPipeMessage.serialize()};

                  DownstreamPacket pkt({id_,dst,0,now},serialization.c_str(),serialization.size());

                  pkt.prependLengthPrefixFraming(serialization.size());

                  if(auto antennaIter = antennas_.find(configuration.getTransmitAntennaIndex());
                     antennaIter != antennas_.end())
                    {
                      Antenna txAntenna{antennaIter->second};

                      txAntenna.setBandwidthHz(configuration.getTransmitBandwidthHz());

                      txAntenna.setFrequencyGroupIndex(0);

                      LOGGER_STANDARD_LOGGING(pPlatformService_->logService(),
                                              DEBUG_LEVEL,
                                              "MACI %03hu BendPipe::RadioModel::%s"
                                              " transponder:%hu tx on antenna index:%hu"
                                              " sending @ %zu hz to dst %hu",
                                              id_,
                                              __func__,
                                              pTransponder->getIndex(),
                                              configuration.getTransmitAntennaIndex(),
                                              configuration.getTransmitFrequencyHz(),
                                              dst);

                      sendDownstreamPacket(CommonMACHeader{REGISTERED_EMANE_MAC_BENT_PIPE,u64SequenceNumber_++},
                                           pkt,
                                           {Controls::MIMOTransmitPropertiesControlMessage::create(FrequencyGroups{{{configuration.getTransmitFrequencyHz(),
                                                                                                                         configuration.getTransmitPowerdBm(),
                                                                                                                         durationMicroseconds}}},
                                               {std::move(txAntenna)}),
                                            Controls::TimeStampControlMessage::create(bentPipeMessage.getStartOfTransmission())});

                    }
                  else
                    {
                      LOGGER_STANDARD_LOGGING(pPlatformService_->logService(),
                                              ERROR_LEVEL,
                                              "MACI %03hu BendPipe::RadioModel::%s"
                                              " transponder:%hu using unknown antenna index:%hu",
                                              id_,
                                              __func__,
                                              pTransponder->getIndex(),
                                              configuration.getTransmitAntennaIndex());

                    }
                }
              else
                {
                  LOGGER_STANDARD_LOGGING(pPlatformService_->logService(),
                                          DEBUG_LEVEL,
                                          "MACI %03hu BendPipe::RadioModel::%s"
                                          " transponder:%hu tx on antenna index:%hu"
                                          " sending @ %zu hz to dst %hu, tx off, drop",
                                          id_,
                                          __func__,
                                          pTransponder->getIndex(),
                                          configuration.getTransmitAntennaIndex(),
                                          configuration.getTransmitFrequencyHz(),
                                          dst);

                  packetStatusPublisher_.outbound(id_,
                                                  bentPipeMessage.getMessages(),
                                                  PacketStatusPublisher::OutboundAction::DROP_TX_OFF);

                }
            }
        }
    }
}

void EMANE::Models::BentPipe::RadioModel::notifyTxOpportunity(Transponder *)
{
  auto now = Clock::now();

  // at least one transponder has a new tx opportunity, check them all
  // and proceed approproatly if there is pending data to transmit.
  process(now);
}


void EMANE::Models::BentPipe::RadioModel::ubendPacket(DownstreamPacket & pkt,
                                                      TransponderIndex transponderIndex)
{
  LOGGER_STANDARD_LOGGING(pPlatformService_->logService(),
                          DEBUG_LEVEL,
                          "MACI %03hu BentPipe::RadioModel::%s ubend transponder %hu",
                          id_,
                          __func__,
                          transponderIndex);


  auto now = Clock::now();

  auto & configuration = transponders_[transponderIndex]->getConfiguration();

  auto & delayMicroseconds = configuration.getTransmitUbendDelay();

  if(delayMicroseconds > Microseconds::zero())
    {
      pPlatformService_->timerService().
        schedule([this,
                  pkt=std::move(pkt),
                  transponderIndex] (const TimePoint &,
                                     const TimePoint &,
                                     const TimePoint &) mutable
        {
          auto now = Clock::now();

          queueManager_.enqueue(transponderIndex,std::move(pkt));

          process(now);
        },
                 now + delayMicroseconds);
    }
  else
    {
      queueManager_.enqueue(transponderIndex,std::move(pkt));

      process(now);
    }
}

void EMANE::Models::BentPipe::RadioModel::processPacket(UpstreamPacket & pkt)
{
  const PacketInfo & pktInfo{pkt.getPacketInfo()};

  LOGGER_VERBOSE_LOGGING(pPlatformService_->logService(),
                         DEBUG_LEVEL,
                         "MACI %03hu BentPipe::RadioModel::%s (upstream) src %hu dst %hu tos %hhu",
                         id_,
                         __func__,
                         pktInfo.getSource(),
                         pktInfo.getDestination(),
                         pktInfo.getPriority());

  sendUpstreamPacket(pkt);
}

DECLARE_MAC_LAYER(EMANE::Models::BentPipe::RadioModel);




extern "C" {
    void emane_bentpipe_radiomodel_processUpstreamControl(void* radiomodel, const void* msgs);
    void emane_bentpipe_radiomodel_processDownstreamControl(void* radiomodel, const void* msgs);
    void emane_bentpipe_radiomodel_processUpstreamPacket(void* radiomodel, const void* hdr, void* pkt, const void* msgs);
    void emane_bentpipe_radiomodel_processDownstreamPacket(void* radiomodel, void* pkt, const void* msgs);

    uint16_t emane_bentpipe_upstreampacket_get_src(const void* pkt) {
        return static_cast<const EMANE::UpstreamPacket*>(pkt)->getPacketInfo().getSource();
    }
    uint16_t emane_bentpipe_upstreampacket_get_dst(const void* pkt) {
        return static_cast<const EMANE::UpstreamPacket*>(pkt)->getPacketInfo().getDestination();
    }
    uint8_t emane_bentpipe_upstreampacket_get_priority(const void* pkt) {
        return static_cast<const EMANE::UpstreamPacket*>(pkt)->getPacketInfo().getPriority();
    }
    size_t emane_bentpipe_upstreampacket_length(const void* pkt) {
        return static_cast<const EMANE::UpstreamPacket*>(pkt)->length();
    }
    size_t emane_bentpipe_upstreampacket_stripLengthPrefixFraming(void* pkt) {
        return static_cast<EMANE::UpstreamPacket*>(pkt)->stripLengthPrefixFraming();
    }
    const void* emane_bentpipe_upstreampacket_get(const void* pkt) {
        return static_cast<const EMANE::UpstreamPacket*>(pkt)->get();
    }
    uint8_t emane_bentpipe_downstreampacket_get_priority(const void* pkt) {
        return static_cast<const EMANE::DownstreamPacket*>(pkt)->getPacketInfo().getPriority();
    }
    uint16_t emane_bentpipe_commonmacheader_get_registration_id(const void* hdr) {
        return static_cast<const EMANE::CommonMACHeader*>(hdr)->getRegistrationId();
    }
    uint64_t emane_bentpipe_commonmacheader_get_sequence_number(const void* hdr) {
        return static_cast<const EMANE::CommonMACHeader*>(hdr)->getSequenceNumber();
    }

    void emane_bentpipe_radiomodel_drop_registration_id(void* radiomodel, uint16_t src, uint16_t dst, size_t length) {
        auto rm = static_cast<EMANE::Models::BentPipe::RadioModel*>(radiomodel);
        rm->packetStatusPublisher_.inbound(src, dst, length, EMANE::Models::BentPipe::PacketStatusPublisher::InboundAction::DROP_REGISTRATION_ID);
    }
    
    void emane_bentpipe_radiomodel_drop_rx_off(void* radiomodel, uint16_t src, const void* pkt_data, size_t pkt_len) {
        auto rm = static_cast<EMANE::Models::BentPipe::RadioModel*>(radiomodel);
        EMANE::Models::BentPipe::BentPipeMessage bentPipeMessage{pkt_data, pkt_len};
        rm->packetStatusPublisher_.inbound(src, bentPipeMessage.getMessages(), EMANE::Models::BentPipe::PacketStatusPublisher::InboundAction::DROP_RX_OFF);
    }
    
    int32_t emane_bentpipe_radiomodel_find_transponder(void* radiomodel, const void* msgs_ptr, uint16_t* out_antenna_index, uint64_t* out_freq_hz, uint64_t* out_span, uint64_t* out_start_of_reception) {
        auto rm = static_cast<EMANE::Models::BentPipe::RadioModel*>(radiomodel);
        auto msgs = static_cast<const EMANE::ControlMessages*>(msgs_ptr);
        
        const EMANE::Controls::MIMOReceivePropertiesControlMessage * pMIMOReceivePropertiesControlMessage{};
        for(auto & pControlMessage : *msgs) {
            if(pControlMessage->getId() == EMANE::Controls::MIMOReceivePropertiesControlMessage::IDENTIFIER) {
                pMIMOReceivePropertiesControlMessage = static_cast<const EMANE::Controls::MIMOReceivePropertiesControlMessage *>(pControlMessage);
                break;
            }
        }
        if(!pMIMOReceivePropertiesControlMessage) return -1;
        
        const EMANE::Controls::AntennaReceiveInfo & antennaReceiveInfo = *pMIMOReceivePropertiesControlMessage->getAntennaReceiveInfos().begin();
        const EMANE::FrequencySegments & frequencySegments = antennaReceiveInfo.getFrequencySegments();
        const EMANE::FrequencySegment & frequencySegment = *frequencySegments.begin();
        
        auto iter = rm->antennaTransponderMap_.find(std::make_pair(antennaReceiveInfo.getRxAntennaIndex(), frequencySegment.getFrequencyHz()));
        if(iter == rm->antennaTransponderMap_.end()) return -1;
        
        *out_antenna_index = antennaReceiveInfo.getRxAntennaIndex();
        *out_freq_hz = frequencySegment.getFrequencyHz();
        *out_span = antennaReceiveInfo.getSpan().count();
        *out_start_of_reception = std::chrono::duration_cast<std::chrono::microseconds>(
            (pMIMOReceivePropertiesControlMessage->getTxTime() +
             pMIMOReceivePropertiesControlMessage->getPropagationDelay() +
             frequencySegment.getOffset()).time_since_epoch()).count();
             
        return iter->second;
    }
    
    void emane_bentpipe_radiomodel_enqueue_receive(void* radiomodel, int32_t transponder_index, void* pkt_ptr, const void* msgs_ptr, uint64_t seq_num) {
        auto rm = static_cast<EMANE::Models::BentPipe::RadioModel*>(radiomodel);
        auto pkt = static_cast<EMANE::UpstreamPacket*>(pkt_ptr);
        auto msgs = static_cast<const EMANE::ControlMessages*>(msgs_ptr);
        
        const EMANE::Controls::MIMOReceivePropertiesControlMessage * pMIMOReceivePropertiesControlMessage{};
        for(auto & pControlMessage : *msgs) {
            if(pControlMessage->getId() == EMANE::Controls::MIMOReceivePropertiesControlMessage::IDENTIFIER) {
                pMIMOReceivePropertiesControlMessage = static_cast<const EMANE::Controls::MIMOReceivePropertiesControlMessage *>(pControlMessage);
                break;
            }
        }
        if(!pMIMOReceivePropertiesControlMessage) return;
        
        const EMANE::Controls::AntennaReceiveInfo & antennaReceiveInfo = *pMIMOReceivePropertiesControlMessage->getAntennaReceiveInfos().begin();
        const EMANE::FrequencySegments & frequencySegments = antennaReceiveInfo.getFrequencySegments();
        const EMANE::FrequencySegment & frequencySegment = *frequencySegments.begin();
        
        EMANE::TimePoint startOfReception{pMIMOReceivePropertiesControlMessage->getTxTime() +
                                   pMIMOReceivePropertiesControlMessage->getPropagationDelay() +
                                   frequencySegment.getOffset()};
                                   
        auto len = pkt->length();
        EMANE::Models::BentPipe::BentPipeMessage bentPipeMessage{pkt->get(), len};
        
        auto & configuration = rm->transponders_[transponder_index]->getConfiguration();
        if(configuration.getReceiveEnable()) {
            rm->receiveManagers_[transponder_index]->enqueue(std::move(bentPipeMessage),
                                                             pkt->getPacketInfo(),
                                                             pkt->length(),
                                                             startOfReception,
                                                             frequencySegments,
                                                             antennaReceiveInfo.getSpan(),
                                                             EMANE::Clock::now(),
                                                             seq_num);
        } else {
            rm->packetStatusPublisher_.inbound(pkt->getPacketInfo().getSource(),
                                               bentPipeMessage.getMessages(),
                                               EMANE::Models::BentPipe::PacketStatusPublisher::InboundAction::DROP_RX_OFF);
        }
    }
    
    int32_t emane_bentpipe_radiomodel_get_tos_to_transponder(void* radiomodel, uint8_t tos) {
        auto rm = static_cast<EMANE::Models::BentPipe::RadioModel*>(radiomodel);
        return rm->tosToTransponder_[tos];
    }
    
    void emane_bentpipe_radiomodel_queue_manager_enqueue(void* radiomodel, int32_t transponder_index, void* pkt_ptr) {
        auto rm = static_cast<EMANE::Models::BentPipe::RadioModel*>(radiomodel);
        auto pkt = static_cast<EMANE::DownstreamPacket*>(pkt_ptr);
        rm->queueManager_.enqueue(transponder_index, std::move(*pkt));
    }
    
    void emane_bentpipe_radiomodel_process(void* radiomodel) {
        auto rm = static_cast<EMANE::Models::BentPipe::RadioModel*>(radiomodel);
        rm->process(EMANE::Clock::now());
    }
}
