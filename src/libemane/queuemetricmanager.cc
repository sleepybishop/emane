#include "emane/queuemetricmanager.h"

extern "C" {
    void* emane_rs_queue_metric_manager_create(uint16_t nem_id);
    void emane_rs_queue_metric_manager_destroy(void* ptr);
    void emane_rs_queue_metric_manager_update(void* ptr, uint16_t queue_id, uint32_t max_queue_size, uint32_t current_queue_depth, uint32_t num_discards, uint64_t delay_microseconds);
    
    struct FfiR2RIQueueMetric {
        uint16_t queue_id;
        uint32_t queue_max_size;
        uint32_t queue_current_depth_high_water;
        uint32_t num_discards_high_water;
        uint64_t avg_delay_microseconds;
    };
    
    FfiR2RIQueueMetric* emane_rs_queue_metric_manager_get(void* ptr, size_t* out_len);
    void emane_rs_queue_metric_manager_free(FfiR2RIQueueMetric* ptr, size_t len);
    bool emane_rs_queue_metric_manager_add(void* ptr, uint16_t queue_id, uint32_t max_queue_size);
    bool emane_rs_queue_metric_manager_remove(void* ptr, uint16_t queue_id);
}

class EMANE::QueueMetricManager::Implementation
{
public:
  Implementation(EMANE::NEMId nemId) :
    pRsManager_{emane_rs_queue_metric_manager_create(nemId)}
  { }

  ~Implementation()
  {
    if(pRsManager_) {
        emane_rs_queue_metric_manager_destroy(pRsManager_);
    }
  }

  void updateQueueMetric(std::uint16_t u16QueueId,
                         std::uint32_t u32MaxQueueSize,
                         std::uint32_t u32CurrentQueueDepth,
                         std::uint32_t u32NumDiscards,
                         const Microseconds & delayMicroseconds)
  {
    emane_rs_queue_metric_manager_update(pRsManager_, u16QueueId, u32MaxQueueSize, u32CurrentQueueDepth, u32NumDiscards, delayMicroseconds.count());
  }

  EMANE::Controls::R2RIQueueMetrics getQueueMetrics()
  {
    Controls::R2RIQueueMetrics metrics;
    size_t out_len = 0;
    
    FfiR2RIQueueMetric* ffi_metrics = emane_rs_queue_metric_manager_get(pRsManager_, &out_len);
    
    if(ffi_metrics) {
        for(size_t i = 0; i < out_len; ++i) {
            metrics.push_back(Controls::R2RIQueueMetric{
                ffi_metrics[i].queue_id,
                ffi_metrics[i].queue_max_size,
                ffi_metrics[i].queue_current_depth_high_water,
                ffi_metrics[i].num_discards_high_water,
                Microseconds{ffi_metrics[i].avg_delay_microseconds}
            });
        }
        emane_rs_queue_metric_manager_free(ffi_metrics, out_len);
    }
    
    return metrics;
  }

  bool addQueueMetric(std::uint16_t u16QueueId, std::uint32_t u32MaxQueueSize)
  {
    return emane_rs_queue_metric_manager_add(pRsManager_, u16QueueId, u32MaxQueueSize);
  }

  bool removeQueueMetric(std::uint16_t u16QueueId)
  {
    return emane_rs_queue_metric_manager_remove(pRsManager_, u16QueueId);
  }

private:
  void* pRsManager_;
};

EMANE::QueueMetricManager::QueueMetricManager(EMANE::NEMId nemId) :
  pImpl_{new Implementation{nemId}}
{ }

EMANE::QueueMetricManager::~QueueMetricManager()
{ }

void
EMANE::QueueMetricManager::updateQueueMetric(std::uint16_t u16QueueId,
                                             std::uint32_t u32MaxQueueSize,
                                             std::uint32_t u32CurrentQueueDepth,
                                             std::uint32_t u32NumDiscards,
                                             const Microseconds & delayMicroseconds)
{
  pImpl_->updateQueueMetric(u16QueueId, u32MaxQueueSize, u32CurrentQueueDepth, u32NumDiscards, delayMicroseconds);
}

EMANE::Controls::R2RIQueueMetrics
EMANE::QueueMetricManager::getQueueMetrics()
{
  return pImpl_->getQueueMetrics();
}

bool EMANE::QueueMetricManager::addQueueMetric(std::uint16_t u16QueueId, std::uint32_t u32MaxQueueSize)
{
  return pImpl_->addQueueMetric(u16QueueId, u32MaxQueueSize);
}

bool EMANE::QueueMetricManager::removeQueueMetric(std::uint16_t u16QueueId)
{
  return pImpl_->removeQueueMetric(u16QueueId);
}
