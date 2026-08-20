/*
 * Copyright (c) 2015,2017-2018,2023 - Adjacent Link LLC, Bridgewater,
 *  New Jersey
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

#include "receivemanager.h"
#include "bentpipemessage.pb.h"
#include "emane/utils/spectrumwindowutils.h"

extern "C" {
  void* emane_rs_bentpipe_receive_manager_new(void* cpp_this, uint16_t id, uint16_t transponder_index, bool b_process, uint16_t rx_antenna_index, uint64_t fragment_check_threshold, uint64_t fragment_timeout_threshold);
  void emane_rs_bentpipe_receive_manager_free(void* rs_rm);
  void emane_rs_bentpipe_receive_manager_enqueue(void* rs_rm, void* msg_ptr, void* pkt_info_ptr, size_t length, uint64_t sor, void* freq_segments_ptr, uint64_t span, uint64_t begin_time, uint64_t seq);
  void emane_rs_bentpipe_receive_manager_process(void* rs_rm);

  uint64_t emane_bentpipe_receivemanager_cxx_now(void* /*cpp_this*/) {
    return std::chrono::duration_cast<std::chrono::microseconds>(EMANE::Clock::now().time_since_epoch()).count();
  }
  void emane_bentpipe_receivemanager_cxx_drop_lock(void* cpp_this, void* pkt_info_ptr, void* msg_ptr) {
    auto rm = static_cast<EMANE::Models::BentPipe::ReceiveManager*>(cpp_this);
    auto pktInfo = static_cast<EMANE::PacketInfo*>(pkt_info_ptr);
    auto msg = static_cast<EMANE::Models::BentPipe::BentPipeMessage*>(msg_ptr);
    rm->pPacketStatusPublisher_->inbound(pktInfo->getSource(), msg->getMessages(), std::remove_pointer<decltype(rm->pPacketStatusPublisher_)>::type::InboundAction::DROP_LOCK);
  }
  void emane_bentpipe_receivemanager_cxx_drop_spectrum_service(void* cpp_this, void* pkt_info_ptr, void* msg_ptr) {
    auto rm = static_cast<EMANE::Models::BentPipe::ReceiveManager*>(cpp_this);
    auto pktInfo = static_cast<EMANE::PacketInfo*>(pkt_info_ptr);
    auto msg = static_cast<EMANE::Models::BentPipe::BentPipeMessage*>(msg_ptr);
    rm->pPacketStatusPublisher_->inbound(pktInfo->getSource(), msg->getMessages(), std::remove_pointer<decltype(rm->pPacketStatusPublisher_)>::type::InboundAction::DROP_SPECTRUM_SERVICE);
  }
  void emane_bentpipe_receivemanager_cxx_drop_sinr(void* cpp_this, void* pkt_info_ptr, void* msg_ptr) {
    auto rm = static_cast<EMANE::Models::BentPipe::ReceiveManager*>(cpp_this);
    auto pktInfo = static_cast<EMANE::PacketInfo*>(pkt_info_ptr);
    auto msg = static_cast<EMANE::Models::BentPipe::BentPipeMessage*>(msg_ptr);
    rm->pPacketStatusPublisher_->inbound(pktInfo->getSource(), msg->getMessages(), std::remove_pointer<decltype(rm->pPacketStatusPublisher_)>::type::InboundAction::DROP_SINR);
  }
  void emane_bentpipe_receivemanager_cxx_drop_bad_curve(void* cpp_this, void* pkt_info_ptr, void* msg_ptr) {
    auto rm = static_cast<EMANE::Models::BentPipe::ReceiveManager*>(cpp_this);
    auto pktInfo = static_cast<EMANE::PacketInfo*>(pkt_info_ptr);
    auto msg = static_cast<EMANE::Models::BentPipe::BentPipeMessage*>(msg_ptr);
    rm->pPacketStatusPublisher_->inbound(pktInfo->getSource(), msg->getMessages(), std::remove_pointer<decltype(rm->pPacketStatusPublisher_)>::type::InboundAction::DROP_BAD_CURVE);
  }
  void emane_bentpipe_receivemanager_cxx_publish_drop_destination_mac(void* cpp_this, void* pkt_info_ptr, void* msg_ptr, size_t msg_index) {
    auto rm = static_cast<EMANE::Models::BentPipe::ReceiveManager*>(cpp_this);
    auto pktInfo = static_cast<EMANE::PacketInfo*>(pkt_info_ptr);
    auto msg = static_cast<EMANE::Models::BentPipe::BentPipeMessage*>(msg_ptr);
    rm->pPacketStatusPublisher_->inbound(pktInfo->getSource(), (*std::next(msg->getMessages().begin(), msg_index)), std::remove_pointer<decltype(rm->pPacketStatusPublisher_)>::type::InboundAction::DROP_DESTINATION_MAC);
  }
  void emane_bentpipe_receivemanager_cxx_publish_accept_good(void* cpp_this, void* pkt_info_ptr, void* msg_ptr, size_t msg_index) {
    auto rm = static_cast<EMANE::Models::BentPipe::ReceiveManager*>(cpp_this);
    auto pktInfo = static_cast<EMANE::PacketInfo*>(pkt_info_ptr);
    auto msg = static_cast<EMANE::Models::BentPipe::BentPipeMessage*>(msg_ptr);
    rm->pPacketStatusPublisher_->inbound(pktInfo->getSource(), (*std::next(msg->getMessages().begin(), msg_index)), std::remove_pointer<decltype(rm->pPacketStatusPublisher_)>::type::InboundAction::ACCEPT_GOOD);
  }
  void emane_bentpipe_receivemanager_cxx_publish_accept_good_len(void* cpp_this, void* pkt_info_ptr, uint16_t dst, size_t length) {
    auto rm = static_cast<EMANE::Models::BentPipe::ReceiveManager*>(cpp_this);
    auto pktInfo = static_cast<EMANE::PacketInfo*>(pkt_info_ptr);
    rm->pPacketStatusPublisher_->inbound(pktInfo->getSource(), dst, length, std::remove_pointer<decltype(rm->pPacketStatusPublisher_)>::type::InboundAction::ACCEPT_GOOD);
  }
  int emane_bentpipe_receivemanager_cxx_check_spectrum(void* cpp_this, uint16_t rx_antenna_index, uint64_t freq_hz, uint64_t span, uint64_t sor, double rx_power_dbm, double* out_noise_floor, bool* out_signal_in_noise) {
    auto rm = static_cast<EMANE::Models::BentPipe::ReceiveManager*>(cpp_this);
    try {
      auto window = rm->pRadioService_->spectrumService().requestAntenna(rx_antenna_index, freq_hz, EMANE::Microseconds{span}, EMANE::TimePoint{EMANE::Microseconds{sor}});
      std::tie(*out_noise_floor, *out_signal_in_noise) = EMANE::Utils::maxBinNoiseFloor(window, rx_power_dbm);
      return 0;
    } catch (EMANE::SpectrumServiceException &) {
      return -1;
    }
  }
  int emane_bentpipe_receivemanager_cxx_get_por(void* cpp_this, uint16_t pcr_curve_index, double sinr, size_t length, float* out_por) {
    auto rm = static_cast<EMANE::Models::BentPipe::ReceiveManager*>(cpp_this);
    if(auto fPOR = rm->pPCRManager_->getPOR(pcr_curve_index, sinr, length)) {
      *out_por = *fPOR;
      return 0;
    }
    return -1;
  }
  float emane_bentpipe_receivemanager_cxx_get_random(void* cpp_this) {
    auto rm = static_cast<EMANE::Models::BentPipe::ReceiveManager*>(cpp_this);
    return rm->distribution_();
  }
  void emane_bentpipe_receivemanager_cxx_update_neighbor_metrics(void* cpp_this, void* pkt_info_ptr, uint16_t transponder_index, double sinr, double noise_floor, uint64_t sor) {
    auto rm = static_cast<EMANE::Models::BentPipe::ReceiveManager*>(cpp_this);
    auto pktInfo = static_cast<EMANE::PacketInfo*>(pkt_info_ptr);
    rm->pNeighborStatusPublisher_->update(pktInfo->getSource(), transponder_index, sinr, noise_floor, EMANE::TimePoint{EMANE::Microseconds{sor}});
  }
  size_t emane_bentpipe_receivemanager_cxx_get_messages_count(void* msg_ptr) {
    return static_cast<EMANE::Models::BentPipe::BentPipeMessage*>(msg_ptr)->getMessages().size();
  }
  uint16_t emane_bentpipe_receivemanager_cxx_msg_get_dst(void* msg_ptr, size_t index) {
    return std::next(static_cast<EMANE::Models::BentPipe::BentPipeMessage*>(msg_ptr)->getMessages().begin(), index)->getDestination();
  }
  bool emane_bentpipe_receivemanager_cxx_msg_is_fragment(void* msg_ptr, size_t index) {
    return std::next(static_cast<EMANE::Models::BentPipe::BentPipeMessage*>(msg_ptr)->getMessages().begin(), index)->isFragment();
  }
  size_t emane_bentpipe_receivemanager_cxx_msg_get_fragment_index(void* msg_ptr, size_t index) {
    return std::next(static_cast<EMANE::Models::BentPipe::BentPipeMessage*>(msg_ptr)->getMessages().begin(), index)->getFragmentIndex();
  }
  size_t emane_bentpipe_receivemanager_cxx_msg_get_fragment_offset(void* msg_ptr, size_t index) {
    return std::next(static_cast<EMANE::Models::BentPipe::BentPipeMessage*>(msg_ptr)->getMessages().begin(), index)->getFragmentOffset();
  }
  uint64_t emane_bentpipe_receivemanager_cxx_msg_get_fragment_sequence(void* msg_ptr, size_t index) {
    return std::next(static_cast<EMANE::Models::BentPipe::BentPipeMessage*>(msg_ptr)->getMessages().begin(), index)->getFragmentSequence();
  }
  bool emane_bentpipe_receivemanager_cxx_msg_is_more_fragments(void* msg_ptr, size_t index) {
    return std::next(static_cast<EMANE::Models::BentPipe::BentPipeMessage*>(msg_ptr)->getMessages().begin(), index)->isMoreFragments();
  }
  const uint8_t* emane_bentpipe_receivemanager_cxx_msg_get_data(void* msg_ptr, size_t index, size_t* out_len) {
    const auto& data = std::next(static_cast<EMANE::Models::BentPipe::BentPipeMessage*>(msg_ptr)->getMessages().begin(), index)->getData();
    *out_len = data.size();
    return reinterpret_cast<const uint8_t*>(data.data());
  }
  uint16_t emane_bentpipe_receivemanager_cxx_pktinfo_get_src(void* pkt_info_ptr) { return static_cast<EMANE::PacketInfo*>(pkt_info_ptr)->getSource(); }
  uint16_t emane_bentpipe_receivemanager_cxx_pktinfo_get_dst(void* pkt_info_ptr) { return static_cast<EMANE::PacketInfo*>(pkt_info_ptr)->getDestination(); }
  uint8_t emane_bentpipe_receivemanager_cxx_pktinfo_get_priority(void* pkt_info_ptr) { return static_cast<EMANE::PacketInfo*>(pkt_info_ptr)->getPriority(); }
  uint64_t emane_bentpipe_receivemanager_cxx_pktinfo_get_creation_time(void* pkt_info_ptr) { return std::chrono::duration_cast<std::chrono::microseconds>(static_cast<EMANE::PacketInfo*>(pkt_info_ptr)->getCreationTime().time_since_epoch()).count(); }
  uint16_t emane_bentpipe_receivemanager_cxx_bpm_get_pcr_curve_index(void* msg_ptr) { return static_cast<EMANE::Models::BentPipe::BentPipeMessage*>(msg_ptr)->getPCRCurveIndex(); }
  uint64_t emane_bentpipe_receivemanager_cxx_freq_get_frequency_hz(void* freq_ptr) { return static_cast<EMANE::FrequencySegments*>(freq_ptr)->begin()->getFrequencyHz(); }
  double emane_bentpipe_receivemanager_cxx_freq_get_rx_power_dbm(void* freq_ptr) { return static_cast<EMANE::FrequencySegments*>(freq_ptr)->begin()->getRxPowerdBm(); }
  void emane_bentpipe_receivemanager_cxx_forward_upstream(void* cpp_this, void* pkt_info_ptr, uint16_t dst, const uint8_t* data, size_t len) {
    auto rm = static_cast<EMANE::Models::BentPipe::ReceiveManager*>(cpp_this);
    auto pktInfo = static_cast<EMANE::PacketInfo*>(pkt_info_ptr);
    EMANE::UpstreamPacket pkt{{pktInfo->getSource(), dst, pktInfo->getPriority(), pktInfo->getCreationTime(), pktInfo->getUUID()}, data, len};
    rm->pTransponderPacketTransport_->processPacket(pkt);
  }
  void emane_bentpipe_receivemanager_cxx_bend_downstream(void* cpp_this, void* pkt_info_ptr, uint16_t dst, const uint8_t* data, size_t len) {
    auto rm = static_cast<EMANE::Models::BentPipe::ReceiveManager*>(cpp_this);
    auto pktInfo = static_cast<EMANE::PacketInfo*>(pkt_info_ptr);
    EMANE::DownstreamPacket pkt{{rm->id_, dst, pktInfo->getPriority(), pktInfo->getCreationTime(), pktInfo->getUUID()}, data, len};
    rm->pTransponderPacketTransport_->ubendPacket(pkt, rm->transponderIndex_);
  }
  void emane_bentpipe_receivemanager_cxx_forward_upstream_parts(void* cpp_this, void* pkt_info_ptr, uint16_t dst, const uint8_t** data_ptrs, const size_t* data_lens, size_t num_parts) {
    auto rm = static_cast<EMANE::Models::BentPipe::ReceiveManager*>(cpp_this);
    auto pktInfo = static_cast<EMANE::PacketInfo*>(pkt_info_ptr);
    EMANE::Utils::VectorIO vectorIO{};
    for(size_t i=0; i<num_parts; ++i) {
      vectorIO.push_back(EMANE::Utils::make_iovec(const_cast<uint8_t*>(data_ptrs[i]), data_lens[i]));
    }
    EMANE::UpstreamPacket pkt{{pktInfo->getSource(), dst, pktInfo->getPriority(), pktInfo->getCreationTime(), pktInfo->getUUID()}, vectorIO};
    rm->pTransponderPacketTransport_->processPacket(pkt);
  }
  void emane_bentpipe_receivemanager_cxx_bend_downstream_parts(void* cpp_this, void* pkt_info_ptr, uint16_t dst, const uint8_t** data_ptrs, const size_t* data_lens, size_t num_parts) {
    auto rm = static_cast<EMANE::Models::BentPipe::ReceiveManager*>(cpp_this);
    auto pktInfo = static_cast<EMANE::PacketInfo*>(pkt_info_ptr);
    EMANE::DownstreamPacket pkt{{rm->id_, dst, pktInfo->getPriority(), pktInfo->getCreationTime(), pktInfo->getUUID()}, nullptr, 0};
    for(size_t i=num_parts; i>0; --i) {
      pkt.prepend(data_ptrs[i-1], data_lens[i-1]);
    }
    rm->pTransponderPacketTransport_->ubendPacket(pkt, rm->transponderIndex_);
  }
  void emane_bentpipe_receivemanager_cxx_schedule_process(void* cpp_this, uint64_t eor) {
    auto rm = static_cast<EMANE::Models::BentPipe::ReceiveManager*>(cpp_this);
    rm->pPlatformService_->timerService().schedule(std::bind(&EMANE::Models::BentPipe::ReceiveManager::process, rm), EMANE::TimePoint{EMANE::Microseconds{eor}});
  }
  void emane_bentpipe_receivemanager_cxx_drop_miss_fragment(void* cpp_this, uint16_t src, uint16_t dst, size_t total_bytes) {
    auto rm = static_cast<EMANE::Models::BentPipe::ReceiveManager*>(cpp_this);
    rm->pPacketStatusPublisher_->inbound(src, dst, total_bytes, std::remove_pointer<decltype(rm->pPacketStatusPublisher_)>::type::InboundAction::DROP_MISS_FRAGMENT);
  }
  void emane_bentpipe_receivemanager_cxx_delete_msg(void* ptr) { delete static_cast<EMANE::Models::BentPipe::BentPipeMessage*>(ptr); }
  void emane_bentpipe_receivemanager_cxx_delete_pkt_info(void* ptr) { delete static_cast<EMANE::PacketInfo*>(ptr); }
  void emane_bentpipe_receivemanager_cxx_delete_freq_segments(void* ptr) { delete static_cast<EMANE::FrequencySegments*>(ptr); }
}
EMANE::Models::BentPipe::ReceiveManager::ReceiveManager(NEMId id,
                                                        TransponderIndex transponderIndex,
                                                        TransponderPacketTransport * pTransponderPacketTransport,
                                                        PlatformServiceProvider * pPlatformService,
                                                        RadioServiceProvider * pRadioService,
                                                        PacketStatusPublisher * pPacketStatusPublisher,
                                                        NeighborStatusPublisher * pNeighborStatusPublisher,
                                                        bool bProcess,
                                                        PCRManager * pPCRManager,
                                                        AntennaIndex rxAntennaIndex,
                                                        const std::chrono::seconds & fragmentCheckThreshold,
                                                        const std::chrono::seconds & fragmentTimeoutThreshold):

                                                        id_{id},
                                                        transponderIndex_{transponderIndex},
                                                        pTransponderPacketTransport_{pTransponderPacketTransport},
                                                        pPlatformService_{pPlatformService},
                                                        pRadioService_{pRadioService},
                                                        pPacketStatusPublisher_{pPacketStatusPublisher},
                                                        pNeighborStatusPublisher_{pNeighborStatusPublisher},
                                                        bProcess_{bProcess},
                                                        pPCRManager_{pPCRManager},
                                                        rxAntennaIndex_{rxAntennaIndex},
                                                        lastEndOfReception_{},
                                                        distribution_{0.0, 1.0},
                                                        fragmentCheckThreshold_{fragmentCheckThreshold},
                                                        fragmentTimeoutThreshold_{fragmentTimeoutThreshold},
                                                        nextEoRCheckTime_{TimePoint::max()},
                                                        p_rust_rm_{nullptr}

{
  LOGGER_STANDARD_LOGGING(pPlatformService_->logService(),
                          DEBUG_LEVEL,
                          "MACI %03hu BentPipe::ReceiveManager::%s created for transponder %hu set for %s",
                          id_,
                          __func__,
                          transponderIndex,
                          bProcess ? "process" : "ubend");

  p_rust_rm_ = emane_rs_bentpipe_receive_manager_new(
      this, id_, transponderIndex_, bProcess_, rxAntennaIndex_,
      std::chrono::duration_cast<std::chrono::microseconds>(fragmentCheckThreshold_).count(),
      std::chrono::duration_cast<std::chrono::microseconds>(fragmentTimeoutThreshold_).count()
  );
}

EMANE::Models::BentPipe::ReceiveManager::~ReceiveManager()
{
  if(p_rust_rm_)
    {
      emane_rs_bentpipe_receive_manager_free(p_rust_rm_);
      p_rust_rm_ = nullptr;
    }
}

void
EMANE::Models::BentPipe::ReceiveManager::enqueue(BentPipeMessage && otaMessage,
                                                 const PacketInfo & pktInfo,
                                                 size_t length,
                                                 const TimePoint & startOfReception,
                                                 const FrequencySegments & frequencySegments,
                                                 const Microseconds & span,
                                                 const TimePoint & beginTime,
                                                 std::uint64_t u64PacketSequence)
{
  BentPipeMessage* msg_ptr = new BentPipeMessage(std::move(otaMessage));
  PacketInfo* pkt_info_ptr = new PacketInfo(pktInfo);
  FrequencySegments* freq_segments_ptr = new FrequencySegments(frequencySegments);
  
  emane_rs_bentpipe_receive_manager_enqueue(
      p_rust_rm_,
      msg_ptr,
      pkt_info_ptr,
      length,
      startOfReception.time_since_epoch().count(),
      freq_segments_ptr,
      span.count(),
      beginTime.time_since_epoch().count(),
      u64PacketSequence
  );
}

void
EMANE::Models::BentPipe::ReceiveManager::process()
{
  emane_rs_bentpipe_receive_manager_process(p_rust_rm_);
}

