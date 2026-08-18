
#include "emane/utils/commonlayerstatistics.h"

extern "C" {
    void* emane_rs_common_layer_statistics_new();
    void emane_rs_common_layer_statistics_destroy(void* ptr);
}

class EMANE::Utils::CommonLayerStatistics::Implementation {
public:
    Implementation(const StatisticTableLabels&, const StatisticTableLabels&, const std::string&) {
        ptr_ = emane_rs_common_layer_statistics_new();
    }
    ~Implementation() {
        emane_rs_common_layer_statistics_destroy(ptr_);
    }
    void registerStatistics(StatisticRegistrar&) {}
    void processInbound(const UpstreamPacket&) {}
    void processInbound(const DownstreamPacket&) {}
    void processOutbound(const UpstreamPacket&, Microseconds, size_t) {}
    void processOutbound(const DownstreamPacket&, Microseconds, size_t, bool) {}
private:
    void* ptr_;
};

EMANE::Utils::CommonLayerStatistics::CommonLayerStatistics(const StatisticTableLabels & unicastDropTableLabels,
                                                           const StatisticTableLabels & broadcastDropTableLabels,
                                                           const std::string & sInstance):
pImpl_{new Implementation{unicastDropTableLabels, broadcastDropTableLabels, sInstance}}
{ }

EMANE::Utils::CommonLayerStatistics::~CommonLayerStatistics() { }

void EMANE::Utils::CommonLayerStatistics::registerStatistics(StatisticRegistrar & statisticRegistrar) { pImpl_->registerStatistics(statisticRegistrar); }
void EMANE::Utils::CommonLayerStatistics::processInbound(const UpstreamPacket & pkt) { pImpl_->processInbound(pkt); }
void EMANE::Utils::CommonLayerStatistics::processOutbound(const UpstreamPacket & pkt, Microseconds delay, size_t dropCode) { pImpl_->processOutbound(pkt, delay, dropCode); }
void EMANE::Utils::CommonLayerStatistics::processInbound(const DownstreamPacket & pkt) { pImpl_->processInbound(pkt); }
void EMANE::Utils::CommonLayerStatistics::processOutbound(const DownstreamPacket & pkt, Microseconds delay, size_t dropCode, bool bSelfGenerated) { pImpl_->processOutbound(pkt, delay, dropCode, bSelfGenerated); }

extern "C" {
    void emane_c_statistic_numeric_add(void* ptr, uint64_t val) {
        if(ptr) {
            auto stat = static_cast<EMANE::StatisticNumeric<std::uint64_t>*>(ptr);
            *stat += val;
        }
    }
    void emane_c_statistic_numeric_set_avg(void* ptr, float val) {
        if(ptr) {
            auto stat = static_cast<EMANE::StatisticNumeric<float>*>(ptr);
            *stat = val;
        }
    }
    void emane_c_statistic_table_set_cell(void* ptr, uint16_t row, int col, uint64_t val) {
        if(ptr) {
            auto table = static_cast<EMANE::StatisticTable<EMANE::NEMId>*>(ptr);
            table->setCell(row, col, EMANE::Any{val});
        }
    }
    void emane_c_statistic_table_add_row(void* ptr, uint16_t row, size_t num_cols) {
        if(ptr) {
            auto table = static_cast<EMANE::StatisticTable<EMANE::NEMId>*>(ptr);
            std::vector<EMANE::Any> v{EMANE::Any{row}};
            v.insert(v.end(), num_cols - 1, EMANE::Any{std::uint64_t{0}});
            table->addRow(row, v);
        }
    }
}
