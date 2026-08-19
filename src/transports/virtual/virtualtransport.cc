#include "virtualtransport.h"
#include "emane/downstreampacket.h"
#include "emane/downstreamtransport.h"
#include "emane/configureexception.h"
#include "emane/startexception.h"
#include "emane/controls/serializedcontrolmessage.h"
#include "emane/controls/flowcontrolcontrolmessage.h"
#include "emane/controls/r2rineighbormetriccontrolmessage.h"
#include "emane/controls/r2rineighbormetriccontrolmessageformatter.h"
#include "emane/controls/r2riqueuemetriccontrolmessage.h"
#include "emane/controls/r2riqueuemetriccontrolmessageformatter.h"
#include "emane/controls/r2riselfmetriccontrolmessage.h"
#include "emane/controls/r2riselfmetriccontrolmessageformatter.h"
#include <sstream>

extern "C" {
    void* emane_rs_virtual_transport_new(uint16_t id, void* cpp_obj, void (*cb)(void*, const uint8_t*, size_t));
    void emane_rs_virtual_transport_free(void* ptr);
    int32_t emane_rs_virtual_transport_start(void* ptr, const char* device_path, const char* device_name, bool arp_mode);
    void emane_rs_virtual_transport_stop(void* ptr);
    int32_t emane_rs_virtual_transport_process_upstream_packet(void* ptr, const uint8_t* buf, size_t len);

    void VirtualTransport_sendDownstreamPacket_cb(void* obj, const uint8_t* buf, size_t len) {
        auto vt = static_cast<EMANE::Transports::Virtual::VirtualTransport*>(obj);
        vt->sendDownstreamPacket_cb(buf, len);
    }
}

namespace {
  const std::uint16_t DROP_CODE_WRITE_ERROR = 1;
  const std::uint16_t DROP_CODE_FRAME_ERROR = 2;
  EMANE::StatisticTableLabels STATISTIC_TABLE_LABELS {"Write Error", "Frame Error"};
}

EMANE::Transports::Virtual::VirtualTransport::VirtualTransport(NEMId id, PlatformServiceProvider * pPlatformService):
  EthernetTransport(id, pPlatformService),
  pBitPool_{},
  flowControlClient_{*this},
  bFlowControlEnable_{},
  commonLayerStatistics_{STATISTIC_TABLE_LABELS},
  rust_obj_{nullptr}
{
    rust_obj_ = emane_rs_virtual_transport_new(id, this, VirtualTransport_sendDownstreamPacket_cb);
}

EMANE::Transports::Virtual::VirtualTransport::~VirtualTransport()
{
  if (rust_obj_) {
      emane_rs_virtual_transport_free(rust_obj_);
      rust_obj_ = nullptr;
  }
  if(pBitPool_) {
      delete pBitPool_;
      pBitPool_ = nullptr;
  }
}

void EMANE::Transports::Virtual::VirtualTransport::initialize(Registrar & registrar)
{
  pBitPool_ = new Utils::BitPool{pPlatformService_, id_};
  auto & configRegistrar = registrar.configurationRegistrar();

  configRegistrar.registerNonNumeric<std::string>("device", ConfigurationProperties::DEFAULT, {"emane0"}, "Virtual device name.");
  configRegistrar.registerNonNumeric<std::string>("devicepath", ConfigurationProperties::DEFAULT, {"/dev/net/tun"}, "Path to the tuntap device.");
  configRegistrar.registerNumeric<std::uint64_t>("bitrate", ConfigurationProperties::DEFAULT, {0}, "Transport bitrate in bps.");
  configRegistrar.registerNumeric<bool>("broadcastmodeenable", ConfigurationProperties::DEFAULT, {false}, "Broadcast all packets.");
  configRegistrar.registerNumeric<bool>("arpcacheenable", ConfigurationProperties::DEFAULT, {true}, "Enable ARP request/reply monitoring.");
  configRegistrar.registerNumeric<bool>("arpmodeenable", ConfigurationProperties::DEFAULT, {true}, "Enable ARP on the virtual device.");
  configRegistrar.registerNumeric<bool>("flowcontrolenable", ConfigurationProperties::DEFAULT, {false}, "Enables downstream traffic flow control.");
  configRegistrar.registerNonNumeric<INETAddr>("address", ConfigurationProperties::NONE, {}, "IPv4 or IPv6 virutal device address.");
  configRegistrar.registerNonNumeric<INETAddr>("mask", ConfigurationProperties::NONE, {}, "IPv4 or IPv6 virutal device addres network mask.");
  configRegistrar.registerNonNumeric<std::string>("ethernet.type.unknown.priority", ConfigurationProperties::NONE, {}, "Unknown eth type priority.", 0, std::numeric_limits<std::uint16_t>::max(), "^(0[xX]){0,1}\\d+:\\d+$");
  configRegistrar.registerNumeric<std::uint8_t>("ethernet.type.arp.priority", ConfigurationProperties::DEFAULT, {0}, "ARP eth priority.");

  auto & statisticRegistrar = registrar.statisticRegistrar();
  commonLayerStatistics_.registerStatistics(statisticRegistrar);
}

void EMANE::Transports::Virtual::VirtualTransport::configure(const ConfigurationUpdate & update)
{
  for(const auto & item : update) {
      if(item.first == "bitrate") { u64BitRate_ = item.second[0].asUINT64(); }
      else if(item.first == "devicepath") { sDevicePath_ = item.second[0].asString(); }
      else if(item.first == "device") { sDeviceName_ = item.second[0].asString(); }
      else if(item.first == "mask") { mask_ = item.second[0].asINETAddr(); }
      else if(item.first == "address") { address_ = item.second[0].asINETAddr(); }
      else if(item.first == "arpmodeenable") { bARPMode_ = item.second[0].asBool(); }
      else if(item.first == "broadcastmodeenable") { bBroadcastMode_ = item.second[0].asBool(); }
      else if(item.first == "arpcacheenable") { bArpCacheMode_ = item.second[0].asBool(); }
      else if(item.first == "flowcontrolenable") { bFlowControlEnable_ = item.second[0].asBool(); }
      else if(item.first == "ethernet.type.arp.priority") { u8EtherTypeARPPriority_ = item.second[0].asUINT8(); }
      else if(item.first == "ethernet.type.unknown.priority") {}
  }
}

void EMANE::Transports::Virtual::VirtualTransport::start()
{
  if (emane_rs_virtual_transport_start(rust_obj_, sDevicePath_.c_str(), sDeviceName_.c_str(), bARPMode_) < 0) {
      throw StartException("Failed to start rust virtual transport");
  }
  pBitPool_->setMaxSize(u64BitRate_);
}

void EMANE::Transports::Virtual::VirtualTransport::postStart()
{
  if(bFlowControlEnable_) {
      flowControlClient_.start();
  }
}

void EMANE::Transports::Virtual::VirtualTransport::stop()
{
  emane_rs_virtual_transport_stop(rust_obj_);
  if(bFlowControlEnable_) {
      flowControlClient_.stop();
  }
}

void EMANE::Transports::Virtual::VirtualTransport::destroy() throw() {}

void EMANE::Transports::Virtual::VirtualTransport::processUpstreamPacket(UpstreamPacket & pkt, const ControlMessages & msgs)
{
  const TimePoint beginTime{Clock::now()};
  if(verifyFrame(pkt.get(), pkt.length()) < 0) {}
  handleUpstreamControl(msgs);
  commonLayerStatistics_.processInbound(pkt);
  
  const PacketInfo & pktInfo{pkt.getPacketInfo()};
  const Utils::EtherHeader * pEtherHeader {reinterpret_cast<const Utils::EtherHeader*>(pkt.get())};
  updateArpCache(pEtherHeader, pktInfo.getSource());

  if (emane_rs_virtual_transport_process_upstream_packet(rust_obj_, reinterpret_cast<const uint8_t*>(pkt.get()), pkt.length()) < 0) {
      commonLayerStatistics_.processOutbound(pkt, std::chrono::duration_cast<Microseconds>(Clock::now() - beginTime), DROP_CODE_WRITE_ERROR);
  } else {
      commonLayerStatistics_.processOutbound(pkt, std::chrono::duration_cast<Microseconds>(Clock::now() - beginTime));
      if(u64BitRate_) { pBitPool_->get(pkt.length() * 8); }
  }
}

void EMANE::Transports::Virtual::VirtualTransport::processUpstreamControl(const ControlMessages & msgs)
{
  handleUpstreamControl(msgs);
}

void EMANE::Transports::Virtual::VirtualTransport::handleUpstreamControl(const ControlMessages & msgs)
{
  for(const auto & pMessage : msgs) {
      if (pMessage->getId() == Controls::FlowControlControlMessage::IDENTIFIER) {
          const auto pFlowControlControlMessage = static_cast<const Controls::FlowControlControlMessage *>(pMessage);
          if(bFlowControlEnable_) {
              flowControlClient_.processFlowControlMessage(pFlowControlControlMessage);
          }
      }
  }
}

void EMANE::Transports::Virtual::VirtualTransport::sendDownstreamPacket_cb(const uint8_t* buf, size_t len)
{
  const TimePoint beginTime{Clock::now()};
  if(verifyFrame(buf, len) < 0) return;

  NEMId nemDestination;
  std::uint8_t dscp{};
  if(parseFrame((const Utils::EtherHeader *)buf, nemDestination, dscp) < 0) return;

  DownstreamPacket pkt(PacketInfo (id_, nemDestination, dscp, Clock::now()), buf, len);
  commonLayerStatistics_.processInbound(pkt);

  if(bFlowControlEnable_) {
      auto status = flowControlClient_.removeToken();
      if(!status.second) return;
  }

  commonLayerStatistics_.processOutbound(pkt, std::chrono::duration_cast<Microseconds>(Clock::now() - beginTime));
  sendDownstreamPacket(pkt);

  if(u64BitRate_) { pBitPool_->get(len * 8); }
}

DECLARE_TRANSPORT(EMANE::Transports::Virtual::VirtualTransport);
