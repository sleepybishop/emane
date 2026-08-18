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

extern "C" {
    size_t emane_c_common_phy_header_get_frequency_groups_size(const void* hdr_ptr) {
        return static_cast<const EMANE::CommonPHYHeader*>(hdr_ptr)->getFrequencyGroups().size();
    }
    const void* emane_c_common_phy_header_get_frequency_segments_ptr(const void* hdr_ptr, size_t group_idx) {
        return &static_cast<const EMANE::CommonPHYHeader*>(hdr_ptr)->getFrequencyGroups()[group_idx];
    }
    size_t emane_c_frequency_segments_size(const void* segs_ptr) {
        return static_cast<const EMANE::FrequencySegments*>(segs_ptr)->size();
    }
    void emane_c_frequency_segments_get(const void* segs_ptr, size_t seg_idx, uint64_t* freq, double* rx_power, bool* has_power, int64_t* dur, int64_t* offset) {
        auto segs = static_cast<const EMANE::FrequencySegments*>(segs_ptr);
        auto it = segs->begin();
        std::advance(it, seg_idx);
        if(freq) *freq = it->getFrequencyHz();
        if(rx_power) *rx_power = it->getPowerdBm().first;
        if(has_power) *has_power = it->getPowerdBm().second;
        if(dur) *dur = it->getDuration().count();
        if(offset) *offset = it->getOffset().count();
    }
    
    size_t emane_c_common_phy_header_get_transmit_antennas_size(const void* hdr_ptr) {
        return static_cast<const EMANE::CommonPHYHeader*>(hdr_ptr)->getTransmitAntennas().size();
    }
    void emane_c_common_phy_header_get_transmit_antenna(const void* hdr_ptr, size_t idx, size_t* freq_group_idx, uint16_t* ant_idx, uint64_t* bw, uint16_t* mask_idx) {
        auto ant = static_cast<const EMANE::CommonPHYHeader*>(hdr_ptr)->getTransmitAntennas()[idx];
        if(freq_group_idx) *freq_group_idx = ant.getFrequencyGroupIndex();
        if(ant_idx) *ant_idx = ant.getIndex();
        if(bw) *bw = ant.getBandwidthHz();
        if(mask_idx) *mask_idx = ant.getSpectralMaskIndex();
    }

    size_t emane_c_common_phy_header_get_transmitters_size(const void* hdr_ptr) {
        return static_cast<const EMANE::CommonPHYHeader*>(hdr_ptr)->getTransmitters().size();
    }
    void emane_c_common_phy_header_get_transmitter(const void* hdr_ptr, size_t idx, uint16_t* nem_id, double* power_dbm) {
        auto txs = static_cast<const EMANE::CommonPHYHeader*>(hdr_ptr)->getTransmitters();
        auto it = txs.begin();
        std::advance(it, idx);
        auto tx = *it;
        if(nem_id) *nem_id = tx.getNEMId();
        if(power_dbm) *power_dbm = tx.getPowerdBm();
    }

    uint64_t emane_c_common_phy_header_get_tx_time(const void* hdr_ptr) {
        return std::chrono::duration_cast<std::chrono::microseconds>(
            static_cast<const EMANE::CommonPHYHeader*>(hdr_ptr)->getTxTime().time_since_epoch()).count();
    }
    uint16_t emane_c_common_phy_header_get_sub_id(const void* hdr_ptr) {
        return static_cast<const EMANE::CommonPHYHeader*>(hdr_ptr)->getSubId();
    }
    const void* emane_c_common_phy_header_get_optional_filter_data(const void* hdr_ptr, bool* out_has_data, size_t* out_len) {
        auto pair = static_cast<const EMANE::CommonPHYHeader*>(hdr_ptr)->getOptionalFilterData();
        if(out_has_data) *out_has_data = pair.second;
        if(out_len) *out_len = pair.first.size();
        return pair.first.data();
    }

    size_t emane_c_antenna_interferences_size(const void* ptr) {
        return static_cast<const EMANE::Controls::AntennaSelfInterferences*>(ptr)->size();
    }
    void emane_c_antenna_interferences_get(const void* ptr, size_t idx, size_t* freq_group_idx, const double** out_powers, size_t* out_powers_len) {
        auto arr = static_cast<const EMANE::Controls::AntennaSelfInterferences*>(ptr);
        auto it = arr->begin();
        std::advance(it, idx);
        if(freq_group_idx) *freq_group_idx = it->getFrequencyGroupIndex();
        if(out_powers) *out_powers = it->getPowerMilliWatts().data();
        if(out_powers_len) *out_powers_len = it->getPowerMilliWatts().size();
    }
    
    size_t emane_c_frequency_groups_size(const void* ptr) {
        return static_cast<const EMANE::FrequencyGroups*>(ptr)->size();
    }
    const void* emane_c_frequency_groups_get_segments_ptr(const void* ptr, size_t idx) {
        return &(*static_cast<const EMANE::FrequencyGroups*>(ptr))[idx];
    }
}

extern "C" {
    struct EMANE_PathlossResult {
        double* pathlosses;
        size_t count;
        bool success;
    };

    EMANE_PathlossResult emane_c_propagation_model_compute(void* algo_ptr,
                                                           uint16_t src,
                                                           const void* loc_info_ptr,
                                                           const void* segments_ptr) {
        auto algo = static_cast<EMANE::PropagationModelAlgorithm*>(algo_ptr);
        auto loc = static_cast<const EMANE::LocationInfo*>(loc_info_ptr);
        auto segs = static_cast<const EMANE::FrequencySegments*>(segments_ptr);
        
        auto res = (*algo)(src, *loc, *segs);
        
        double* arr = nullptr;
        if (res.second && !res.first.empty()) {
            arr = new double[res.first.size()];
            for (size_t i = 0; i < res.first.size(); ++i) {
                arr[i] = res.first[i];
            }
        }
        
        return {
            arr,
            res.first.size(),
            res.second
        };
    }
    
    void emane_c_propagation_model_free_result(EMANE_PathlossResult* res) {
        if (res && res->pathlosses) {
            delete[] res->pathlosses;
        }
    }

}
