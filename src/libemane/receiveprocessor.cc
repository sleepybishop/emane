#include "receiveprocessor.h"
#include "emane/spectrumserviceexception.h"

extern "C" {
    void* emane_rs_receive_processor_create(
        std::uint16_t id,
        std::uint16_t sub_id,
        std::uint16_t rx_antenna_index,
        void* antenna_manager,
        void* spectrum_monitor,
        void* propagation_model,
        void* fading_algorithm_store,
        bool populate_receive_power_map,
        bool populate_observed_power_map,
        bool doppler_shift
    );
    void emane_rs_receive_processor_destroy(void* ptr);
    void emane_rs_receive_processor_process(
        void* rs_ptr,
        std::int64_t now_usec,
        const void* common_phy_header,
        const void* location_infos,
        const void* fading_infos,
        bool in_band,
        void* result_ptr
    );
    void emane_rs_receive_processor_process_self_interference(
        void* rs_ptr,
        std::int64_t now_usec,
        std::int64_t tx_time_usec,
        const void* frequency_groups,
        std::uint64_t segment_bandwidth_hz,
        const void* antenna_interferences,
        const void* optional_filter_data,
        void* result_ptr
    );
}

EMANE::ReceiveProcessor::ReceiveProcessor(NEMId id,
                                          std::uint16_t u16SubId,
                                          AntennaIndex rxAntennaIndex,
                                          AntennaManager & antennaManager,
                                          SpectrumMonitor * pSpectrumMonitor,
                                          PropagationModelAlgorithm * pPropagationModelAlgorithm,
                                          FadingAlgorithmStore && fadingAlgorithmStore,
                                          bool bPopulateReceivePowerMap,
                                          bool bPopulateObservedPowerMap,
                                          bool bDopplerShift):
  rs_ptr_{emane_rs_receive_processor_create(
      id, u16SubId, rxAntennaIndex,
      &antennaManager,
      pSpectrumMonitor,
      pPropagationModelAlgorithm,
      new FadingAlgorithmStore(std::move(fadingAlgorithmStore)),
      bPopulateReceivePowerMap,
      bPopulateObservedPowerMap,
      bDopplerShift)}
{}

EMANE::ReceiveProcessor::~ReceiveProcessor()
{
    emane_rs_receive_processor_destroy(rs_ptr_);
}

EMANE::ReceiveProcessor::ProcessResult
EMANE::ReceiveProcessor::process(const TimePoint & now,
                                 const CommonPHYHeader & commonPHYHeader,
                                 const std::vector<std::pair<LocationInfo,bool>> & locationInfos,
                                 const std::vector<std::pair<FadingInfo,bool>> & fadingInfos,
                                 bool bInBand)
{
  ProcessResult result{};
  auto now_usec = std::chrono::duration_cast<std::chrono::microseconds>(now.time_since_epoch()).count();
  
  emane_rs_receive_processor_process(
      rs_ptr_, now_usec,
      &commonPHYHeader, &locationInfos, &fadingInfos, bInBand, &result);
      
  return result;
}

EMANE::ReceiveProcessor::ProcessSelfInterferenceResult
EMANE::ReceiveProcessor::processSelfInterference(const TimePoint & now,
                                                 const TimePoint & txTimeStamp,
                                                 const FrequencyGroups & frequencyGroups,
                                                 std::uint64_t u64SegmentBandwidthHz,
                                                 const Controls::AntennaSelfInterferences & antennaInterferences,
                                                 const std::pair<FilterData,bool> & optionalFilterData)
{
  ProcessSelfInterferenceResult result;
  auto now_usec = std::chrono::duration_cast<std::chrono::microseconds>(now.time_since_epoch()).count();
  auto tx_usec = std::chrono::duration_cast<std::chrono::microseconds>(txTimeStamp.time_since_epoch()).count();
  
  emane_rs_receive_processor_process_self_interference(
      rs_ptr_, now_usec, tx_usec,
      &frequencyGroups, u64SegmentBandwidthHz,
      &antennaInterferences, &optionalFilterData,
      &result);

  return result;
}
