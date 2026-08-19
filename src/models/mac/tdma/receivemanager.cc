#include "receivemanager.h"
#include "emane/utils/spectrumwindowutils.h"

// FFI Declarations
struct FFIFrequencySegment {
    uint64_t frequency_hz;
    double rx_power_dbm;
    uint64_t duration_micro;
};

struct FFIReceiveManagerCallbacks {
    void* rm_cpp;
    void (*log_error)(void*, uint16_t, const char*);
    void (*log_debug)(void*, uint16_t, const char*);
    bool (*spectrum_request_and_noise)(void*, uint64_t, uint64_t, uint64_t, double, double*, bool*);
    void (*publish_inbound)(void*, uint16_t, uint16_t, uint8_t, size_t, uint32_t);
    void (*publish_inbound_component)(void*, uint16_t, uint32_t, uint32_t, uint16_t, uint8_t, const uint8_t*, size_t, bool, uint32_t, uint32_t, uint64_t, bool);
    void (*publish_inbound_message)(void*, uint16_t, const void*, uint32_t);
    void (*update_neighbor_rx_metric)(void*, uint16_t, uint64_t, const uint8_t*, double, double, uint64_t, uint64_t, uint64_t);
    void (*send_upstream_packet)(void*, uint16_t, uint16_t, uint8_t, uint64_t, const uint8_t*, const uint8_t*, size_t);
    void (*process_packet_meta_info)(void*, uint16_t, uint64_t, double, double, uint64_t);
    void (*process_scheduler_packet)(void*, uint16_t, uint16_t, uint8_t, uint64_t, const uint8_t*, const uint8_t*, size_t, uint64_t, double, double, uint64_t);
};

extern "C" void* tdma_receivemanager_new(uint16_t id, const FFIReceiveManagerCallbacks* callbacks);
extern "C" void tdma_receivemanager_free(void* rm);
extern "C" void tdma_receivemanager_set_promiscuous_mode(void* rm, bool enable);
extern "C" void tdma_receivemanager_load_curves(void* rm, const char* file);
extern "C" void tdma_receivemanager_set_fragment_check_threshold(void* rm, uint64_t seconds);
extern "C" void tdma_receivemanager_set_fragment_timeout_threshold(void* rm, uint64_t seconds);
extern "C" bool tdma_receivemanager_enqueue(
    void* rm, const uint8_t* bmm_bytes, size_t bmm_len, uint16_t src, uint16_t dst, uint64_t ctime,
    const uint8_t* uuid, size_t length, uint64_t sor, const FFIFrequencySegment* segments, size_t segments_count,
    uint64_t span, uint64_t begin_time, uint64_t seq
);
extern "C" void tdma_receivemanager_process(void* rm, uint64_t u64AbsoluteSlotIndex);

// Callbacks
extern "C" {
    static void cb_log_error(void* ptr, uint16_t id, const char* msg) {
        auto rm = static_cast<EMANE::Models::TDMA::ReceiveManager*>(ptr);
        LOGGER_VERBOSE_LOGGING(*rm->getLogService(), EMANE::ERROR_LEVEL, "%s", msg);
    }
    static void cb_log_debug(void* ptr, uint16_t id, const char* msg) {
        auto rm = static_cast<EMANE::Models::TDMA::ReceiveManager*>(ptr);
        LOGGER_VERBOSE_LOGGING(*rm->getLogService(), EMANE::DEBUG_LEVEL, "%s", msg);
    }
    static bool cb_spectrum_request_and_noise(void* ptr, uint64_t freq, uint64_t span_micro, uint64_t sor_micro, double rx_power_dbm, double* out_noise, bool* out_signal_in_noise) {
        auto rm = static_cast<EMANE::Models::TDMA::ReceiveManager*>(ptr);
        try {
            auto window = rm->getRadioService()->spectrumService().request(
                freq, std::chrono::microseconds(span_micro), EMANE::TimePoint(std::chrono::microseconds(sor_micro))
            );
            std::tie(*out_noise, *out_signal_in_noise) = EMANE::Utils::maxBinNoiseFloor(window, rx_power_dbm);
            return true;
        } catch(EMANE::SpectrumServiceException &) {
            return false;
        }
    }
    static void cb_publish_inbound(void* ptr, uint16_t src, uint16_t dst, uint8_t priority, size_t length, uint32_t action) {
        auto rm = static_cast<EMANE::Models::TDMA::ReceiveManager*>(ptr);
        rm->getPacketStatusPublisher()->inbound(src, dst, priority, length, static_cast<EMANE::Models::TDMA::PacketStatusPublisher::InboundAction>(action));
    }
    static void cb_publish_inbound_component(void* ptr, uint16_t src, uint32_t action, uint32_t msg_type, uint16_t dst, uint8_t priority, const uint8_t* data, size_t data_len, bool is_fragment, uint32_t fragment_index, uint32_t fragment_offset, uint64_t fragment_sequence, bool more_fragments) {
        (void)is_fragment; // Not used in MessageComponent ctor
        auto rm = static_cast<EMANE::Models::TDMA::ReceiveManager*>(ptr);
        EMANE::Utils::VectorIO vectorIO;
        vectorIO.push_back(EMANE::Utils::make_iovec(const_cast<uint8_t*>(data), data_len));
        EMANE::Models::TDMA::MessageComponent mc(
            static_cast<EMANE::Models::TDMA::MessageComponent::Type>(msg_type),
            dst, priority,
            vectorIO, fragment_index, fragment_offset, fragment_sequence, more_fragments
        );
        rm->getPacketStatusPublisher()->inbound(src, mc, static_cast<EMANE::Models::TDMA::PacketStatusPublisher::InboundAction>(action));
    }
    static void cb_publish_inbound_message(void* ptr, uint16_t src, const void* bmm_ptr, uint32_t action) {
        auto rm = static_cast<EMANE::Models::TDMA::ReceiveManager*>(ptr);
        auto bmm = static_cast<const EMANE::Models::TDMA::BaseModelMessage*>(bmm_ptr);
        rm->getPacketStatusPublisher()->inbound(src, bmm->getMessages(), static_cast<EMANE::Models::TDMA::PacketStatusPublisher::InboundAction>(action));
    }
    static void cb_update_neighbor_rx_metric(void* ptr, uint16_t src, uint64_t seq, const uint8_t* uuid, double sinr, double noise, uint64_t sor_micro, uint64_t dur_micro, uint64_t datarate) {
        auto rm = static_cast<EMANE::Models::TDMA::ReceiveManager*>(ptr);
        rm->getNeighborMetricManager()->updateNeighborRxMetric(
            src, seq, *reinterpret_cast<const uuid_t*>(uuid), sinr, noise,
            EMANE::TimePoint(std::chrono::microseconds(sor_micro)),
            std::chrono::microseconds(dur_micro), datarate
        );
    }
    static void cb_send_upstream_packet(void* ptr, uint16_t src, uint16_t dst, uint8_t priority, uint64_t ctime_micro, const uint8_t* uuid, const uint8_t* data, size_t len) {
        auto rm = static_cast<EMANE::Models::TDMA::ReceiveManager*>(ptr);
        EMANE::PacketInfo pktInfo(src, dst, priority, EMANE::TimePoint(std::chrono::microseconds(ctime_micro)), *reinterpret_cast<const uuid_t*>(uuid));
        EMANE::UpstreamPacket pkt(pktInfo, data, len);
        rm->getDownstreamTransport()->sendUpstreamPacket(pkt);
    }
    static void cb_process_packet_meta_info(void* ptr, uint16_t src, uint64_t slot_index, double rx_power, double sinr, uint64_t datarate) {
        auto rm = static_cast<EMANE::Models::TDMA::ReceiveManager*>(ptr);
        EMANE::Models::TDMA::PacketMetaInfo pmi{src, slot_index, rx_power, sinr, datarate};
        rm->getScheduler()->processPacketMetaInfo(pmi);
    }
    static void cb_process_scheduler_packet(void* ptr, uint16_t src, uint16_t dst, uint8_t priority, uint64_t ctime_micro, const uint8_t* uuid, const uint8_t* data, size_t len, uint64_t slot_index, double rx_power, double sinr, uint64_t datarate) {
        auto rm = static_cast<EMANE::Models::TDMA::ReceiveManager*>(ptr);
        EMANE::PacketInfo pktInfo(src, dst, priority, EMANE::TimePoint(std::chrono::microseconds(ctime_micro)), *reinterpret_cast<const uuid_t*>(uuid));
        EMANE::UpstreamPacket pkt(pktInfo, data, len);
        EMANE::Models::TDMA::PacketMetaInfo pmi{src, slot_index, rx_power, sinr, datarate};
        rm->getScheduler()->processSchedulerPacket(pkt, pmi);
    }
}

EMANE::Models::TDMA::ReceiveManager::ReceiveManager(
    NEMId id, DownstreamTransport * pDownstreamTransport, LogServiceProvider * pLogService,
    RadioServiceProvider * pRadioService, Scheduler * pScheduler, PacketStatusPublisher * pPacketStatusPublisher, NeighborMetricManager *pNeighborMetricManager
) :
    id_{id},
    pDownstreamTransport_{pDownstreamTransport},
    pLogService_{pLogService},
    pRadioService_{pRadioService},
    pScheduler_{pScheduler},
    pPacketStatusPublisher_{pPacketStatusPublisher},
    pNeighborMetricManager_{pNeighborMetricManager}
{
    FFIReceiveManagerCallbacks cbs;
    cbs.rm_cpp = this;
    cbs.log_error = cb_log_error;
    cbs.log_debug = cb_log_debug;
    cbs.spectrum_request_and_noise = cb_spectrum_request_and_noise;
    cbs.publish_inbound = cb_publish_inbound;
    cbs.publish_inbound_component = cb_publish_inbound_component;
    cbs.publish_inbound_message = cb_publish_inbound_message;
    cbs.update_neighbor_rx_metric = cb_update_neighbor_rx_metric;
    cbs.send_upstream_packet = cb_send_upstream_packet;
    cbs.process_packet_meta_info = cb_process_packet_meta_info;
    cbs.process_scheduler_packet = cb_process_scheduler_packet;
    rm_ = tdma_receivemanager_new(id, &cbs);
}

EMANE::Models::TDMA::ReceiveManager::~ReceiveManager()
{
    tdma_receivemanager_free(rm_);
}

void EMANE::Models::TDMA::ReceiveManager::setPromiscuousMode(bool bEnable)
{
    tdma_receivemanager_set_promiscuous_mode(rm_, bEnable);
}

void EMANE::Models::TDMA::ReceiveManager::loadCurves(const std::string & sPCRFileName)
{
    tdma_receivemanager_load_curves(rm_, sPCRFileName.c_str());
}

void EMANE::Models::TDMA::ReceiveManager::setFragmentCheckThreshold(const std::chrono::seconds & threshold)
{
    tdma_receivemanager_set_fragment_check_threshold(rm_, threshold.count());
}

void EMANE::Models::TDMA::ReceiveManager::setFragmentTimeoutThreshold(const std::chrono::seconds & threshold)
{
    tdma_receivemanager_set_fragment_timeout_threshold(rm_, threshold.count());
}

bool EMANE::Models::TDMA::ReceiveManager::enqueue(
    BaseModelMessage && baseModelMessage, const PacketInfo & pktInfo, size_t length,
    const TimePoint & startOfReception, const FrequencySegments & frequencySegments,
    const Microseconds & span, const TimePoint & beginTime, std::uint64_t u64PacketSequence
) {
    auto ser = baseModelMessage.serialize();
    std::vector<FFIFrequencySegment> f_segs;
    for (const auto& fs : frequencySegments) {
        f_segs.push_back({
            fs.getFrequencyHz(),
            fs.getRxPowerdBm(),
            static_cast<uint64_t>(fs.getDuration().count())
        });
    }

    return tdma_receivemanager_enqueue(
        rm_, reinterpret_cast<const uint8_t*>(ser.c_str()), ser.size(),
        pktInfo.getSource(), pktInfo.getDestination(),
        std::chrono::duration_cast<std::chrono::microseconds>(pktInfo.getCreationTime().time_since_epoch()).count(),
        reinterpret_cast<const uint8_t*>(&pktInfo.getUUID()), length,
        std::chrono::duration_cast<std::chrono::microseconds>(startOfReception.time_since_epoch()).count(),
        f_segs.data(), f_segs.size(), span.count(),
        std::chrono::duration_cast<std::chrono::microseconds>(beginTime.time_since_epoch()).count(),
        u64PacketSequence
    );
}

void EMANE::Models::TDMA::ReceiveManager::process(std::uint64_t u64AbsoluteSlotIndex)
{
    tdma_receivemanager_process(rm_, u64AbsoluteSlotIndex);
}
