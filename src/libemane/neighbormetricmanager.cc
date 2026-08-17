#include "emane/neighbormetricmanager.h"
#include "emane/statistictable.h"

extern "C" {
    void* emane_rs_neighbor_metric_manager_create(uint16_t nem_id);
    void emane_rs_neighbor_metric_manager_destroy(void* ptr);
    void emane_rs_neighbor_metric_manager_set_delete_time(void* ptr, uint64_t age_usec);
    void emane_rs_neighbor_metric_manager_update_tx(void* ptr, uint16_t dst, uint64_t data_rate_bps, double tx_time_sec);
    void emane_rs_neighbor_metric_manager_update_rx_short(void* ptr, uint16_t src, uint64_t seq_num, const uint8_t* uuid, double rx_time_sec);
    void emane_rs_neighbor_metric_manager_update_rx_long(void* ptr, uint16_t src, uint64_t seq_num, const uint8_t* uuid, double sinr, double noise_floor, double rx_time_sec, uint64_t duration_usec, uint64_t data_rate_bps);
    
    struct FfiR2RINeighborMetric {
        uint16_t neighbor_id;
        uint64_t num_rx_frames;
        uint64_t num_tx_frames;
        uint64_t num_rx_missed_frames;
        uint64_t rx_utilization_microseconds;
        float sinr_avg;
        float sinr_std;
        float noise_floor_avg;
        float noise_floor_stdv;
        uint64_t rx_data_rate_avg;
        uint64_t tx_data_rate_avg;
    };
    
    struct FfiNeighborData {
        uint16_t nem_id;
        uint64_t num_rx_frames;
        uint64_t num_tx_frames;
        uint64_t num_rx_missed_frames;
        uint64_t rx_utilization_microseconds;
        double sinr_avg;
        double sinr_std;
        double noise_floor_avg;
        double noise_floor_stdv;
        uint64_t rx_data_rate_avg;
        uint64_t tx_data_rate_avg;
        double last_rx_time_sec;
        bool have_ever_had_rx_activity;
    };
    
    FfiR2RINeighborMetric* emane_rs_neighbor_metric_manager_get_metrics(void* ptr, double current_time_sec, size_t* out_len);
    void emane_rs_neighbor_metric_manager_free_metrics(FfiR2RINeighborMetric* ptr, size_t len);
    
    FfiNeighborData* emane_rs_neighbor_metric_manager_get_status(void* ptr, double current_time_sec, size_t* out_len);
    void emane_rs_neighbor_metric_manager_free_status(FfiNeighborData* ptr, size_t len);
}

class EMANE::NeighborMetricManager::Implementation
{
public:
  Implementation(EMANE::NEMId nemId) :
    pStatisticNeighborMetricTable_{nullptr},
    pRsManager_{emane_rs_neighbor_metric_manager_create(nemId)}
  { }

  ~Implementation()
  {
    if(pRsManager_) {
        emane_rs_neighbor_metric_manager_destroy(pRsManager_);
    }
  }

  void setNeighborDeleteTimeMicroseconds(const Microseconds & ageMicroseconds)
  {
    emane_rs_neighbor_metric_manager_set_delete_time(pRsManager_, ageMicroseconds.count());
  }

  void handleTxActivity(NEMId dst, std::uint64_t u64DataRatebps, const TimePoint & txTime)
  {
    double tx_sec = std::chrono::duration_cast<DoubleSeconds>(txTime.time_since_epoch()).count();
    emane_rs_neighbor_metric_manager_update_tx(pRsManager_, dst, u64DataRatebps, tx_sec);
  }

  void handleRxActivity(NEMId src,
                        std::uint64_t u64SeqNum,
                        const uuid_t & uuid,
                        const TimePoint & rxTime)
  {
    double rx_sec = std::chrono::duration_cast<DoubleSeconds>(rxTime.time_since_epoch()).count();
    emane_rs_neighbor_metric_manager_update_rx_short(pRsManager_, src, u64SeqNum, uuid, rx_sec);
  }

  void handleRxActivity(NEMId src,
                        std::uint64_t u64SeqNum,
                        const uuid_t & uuid,
                        double dSINR,
                        double dNoiseFloor,
                        const TimePoint & rxTime,
                        const Microseconds & durationMicroseconds,
                        std::uint64_t u64DataRatebps)
  {
    double rx_sec = std::chrono::duration_cast<DoubleSeconds>(rxTime.time_since_epoch()).count();
    emane_rs_neighbor_metric_manager_update_rx_long(pRsManager_, src, u64SeqNum, uuid, dSINR, dNoiseFloor, rx_sec, durationMicroseconds.count(), u64DataRatebps);
  }

  EMANE::Controls::R2RINeighborMetrics getNeighborMetrics()
  {
    Controls::R2RINeighborMetrics r2riMetrics;
    size_t out_len = 0;
    
    double now_sec = std::chrono::duration_cast<DoubleSeconds>(Clock::now().time_since_epoch()).count();
    
    FfiR2RINeighborMetric* metrics = emane_rs_neighbor_metric_manager_get_metrics(pRsManager_, now_sec, &out_len);
    
    if(metrics) {
        for(size_t i = 0; i < out_len; ++i) {
            r2riMetrics.push_back(Controls::R2RINeighborMetric{
                metrics[i].neighbor_id,
                metrics[i].num_rx_frames,
                metrics[i].num_tx_frames,
                metrics[i].num_rx_missed_frames,
                Microseconds{metrics[i].rx_utilization_microseconds},
                metrics[i].sinr_avg,
                metrics[i].sinr_std,
                metrics[i].noise_floor_avg,
                metrics[i].noise_floor_stdv,
                metrics[i].rx_data_rate_avg,
                metrics[i].tx_data_rate_avg
            });
        }
        emane_rs_neighbor_metric_manager_free_metrics(metrics, out_len);
    }
    
    return r2riMetrics;
  }

  void handleNeighborStatusUpdate()
  {
    if(!pStatisticNeighborMetricTable_) return;

    size_t out_len = 0;
    double now_sec = std::chrono::duration_cast<DoubleSeconds>(Clock::now().time_since_epoch()).count();
    
    FfiNeighborData* data = emane_rs_neighbor_metric_manager_get_status(pRsManager_, now_sec, &out_len);
    
    if(data) {
        std::vector<NEMId> to_remove;
        
        // Find existing table keys to see if we need to remove any
        // EMANE doesn't expose easy iteration over StatisticTable keys, but here we can just update cells.
        
        for(size_t i = 0; i < out_len; ++i) {
            auto & m = data[i];
            
            pStatisticNeighborMetricTable_->setCell(m.nem_id, 0, Any{m.num_rx_frames});
            pStatisticNeighborMetricTable_->setCell(m.nem_id, 1, Any{m.num_rx_missed_frames});
            pStatisticNeighborMetricTable_->setCell(m.nem_id, 2, Any{m.rx_utilization_microseconds});
            pStatisticNeighborMetricTable_->setCell(m.nem_id, 3, Any{m.last_rx_time_sec});
            pStatisticNeighborMetricTable_->setCell(m.nem_id, 4, Any{m.sinr_avg});
            pStatisticNeighborMetricTable_->setCell(m.nem_id, 5, Any{m.sinr_std});
            pStatisticNeighborMetricTable_->setCell(m.nem_id, 6, Any{m.noise_floor_avg});
            pStatisticNeighborMetricTable_->setCell(m.nem_id, 7, Any{m.noise_floor_stdv});
            pStatisticNeighborMetricTable_->setCell(m.nem_id, 8, Any{m.rx_data_rate_avg});
        }
        emane_rs_neighbor_metric_manager_free_status(data, out_len);
    }
  }

  void registerStatistics(StatisticRegistrar & statisticRegistrar)
  {
    pStatisticNeighborMetricTable_ =
      statisticRegistrar.registerTable<NEMId>("NeighborMetricTable",
                                              {"RxFrames",
                                                  "MissedFrames",
                                                  "BWConsumptionMcrSec",
                                                  "LastRxTime",
                                                  "SINRAvg",
                                                  "SINRStdv",
                                                  "NoiseFloorAvg",
                                                  "NoiseFloorStdv",
                                                  "RxDataRateAvg"},
                                              StatisticProperties::NONE,
                                              "Neighbor metric table.");
  }

private:
  StatisticTable<NEMId> * pStatisticNeighborMetricTable_;
  void* pRsManager_;
};

EMANE::NeighborMetricManager::NeighborMetricManager(EMANE::NEMId nemId) :
  pImpl_{new Implementation{nemId}}
{ }

EMANE::NeighborMetricManager::~NeighborMetricManager()
{ }

void
EMANE::NeighborMetricManager::setNeighborDeleteTimeMicroseconds(const Microseconds & ageMicroseconds)
{
  pImpl_->setNeighborDeleteTimeMicroseconds(ageMicroseconds);
}

void
EMANE::NeighborMetricManager::updateNeighborTxMetric(NEMId dst, std::uint64_t u64DataRatebps, const TimePoint & txTime)
{
  pImpl_->handleTxActivity(dst, u64DataRatebps, txTime);
}

void
EMANE::NeighborMetricManager::updateNeighborRxMetric(NEMId src,
                                                     std::uint64_t u64SeqNum,
                                                     const uuid_t & uuid,
                                                     const TimePoint & rxTime)
{
  pImpl_->handleRxActivity(src, u64SeqNum, uuid, rxTime);
}

void EMANE::NeighborMetricManager::updateNeighborRxMetric(NEMId src,
                                                          std::uint64_t u64SeqNum,
                                                          const uuid_t & uuid,
                                                          double dSINR,
                                                          double dNoiseFloor,
                                                          const TimePoint & rxTime,
                                                          const Microseconds & durationMicroseconds,
                                                          std::uint64_t u64DataRatebps)
{
  pImpl_->handleRxActivity(src,
                           u64SeqNum,
                           uuid,
                           dSINR,
                           dNoiseFloor,
                           rxTime,
                           durationMicroseconds,
                           u64DataRatebps);
}

EMANE::Controls::R2RINeighborMetrics
EMANE::NeighborMetricManager::getNeighborMetrics()
{
  pImpl_->handleNeighborStatusUpdate();
  return pImpl_->getNeighborMetrics();
}

void EMANE::NeighborMetricManager::updateNeighborStatus()
{
  pImpl_->handleNeighborStatusUpdate();
}

void EMANE::NeighborMetricManager::registerStatistics(StatisticRegistrar & statisticRegistrar)
{
  pImpl_->registerStatistics(statisticRegistrar);
}
