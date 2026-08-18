#include "receiveprocessor.h"
#include "emane/locationinfo.h"

extern "C" {
    size_t emane_c_location_infos_size(const void* vec_ptr) {
        auto vec = static_cast<const std::vector<std::pair<EMANE::LocationInfo, bool>>*>(vec_ptr);
        return vec->size();
    }
    
    const void* emane_c_location_infos_get(const void* vec_ptr, size_t index, bool* out_bool) {
        auto vec = static_cast<const std::vector<std::pair<EMANE::LocationInfo, bool>>*>(vec_ptr);
        if(out_bool) *out_bool = (*vec)[index].second;
        return &(*vec)[index].first;
    }

    size_t emane_c_fading_infos_size(const void* vec_ptr) {
        auto vec = static_cast<const std::vector<std::pair<EMANE::FadingInfo, bool>>*>(vec_ptr);
        return vec->size();
    }
    
    const void* emane_c_fading_infos_get(const void* vec_ptr, size_t index, bool* out_bool, uint32_t* out_model) {
        auto vec = static_cast<const std::vector<std::pair<EMANE::FadingInfo, bool>>*>(vec_ptr);
        if(out_bool) *out_bool = (*vec)[index].second;
        if(out_model) *out_model = static_cast<uint32_t>((*vec)[index].first.first);
        return (*vec)[index].first.second;
    }
}
#include "receiveprocessor.h"
#include <vector>
#include <map>
#include <tuple>

extern "C" {
    void emane_c_receive_processor_add_receive_power(
        void* result_ptr,
        uint16_t src, uint16_t rx_ant, uint16_t tx_ant, uint64_t freq,
        double rx_power, double tx_gain, double rx_gain, double tx_power, double pathloss, double doppler
    ) {
        auto res = static_cast<EMANE::ReceiveProcessor::ProcessResult*>(result_ptr);
        res->receivePowerMap_.insert(std::make_pair(
            std::make_tuple(src, rx_ant, tx_ant, freq),
            std::make_tuple(rx_power, tx_gain, rx_gain, tx_power, pathloss, doppler)
        ));
    }

    void emane_c_receive_processor_add_observed_power(
        void* result_ptr,
        uint16_t src, uint16_t rx_ant, uint16_t tx_ant, uint64_t freq,
        uint16_t mask_index, double rx_power
    ) {
        auto res = static_cast<EMANE::ReceiveProcessor::ProcessResult*>(result_ptr);
        res->observedPowerMap_.insert(std::make_pair(
            std::make_tuple(src, rx_ant, tx_ant, freq),
            std::make_tuple(mask_index, rx_power)
        ));
    }
}

extern "C" {
    void emane_c_receive_processor_set_status(void* result_ptr, uint32_t status) {
        auto res = static_cast<EMANE::ReceiveProcessor::ProcessResult*>(result_ptr);
        res->status_ = static_cast<EMANE::ReceiveProcessor::ProcessResult::Status>(status);
    }
    
    void emane_c_receive_processor_set_mimo(void* result_ptr, uint64_t sot, uint64_t prop_delay) {
        auto res = static_cast<EMANE::ReceiveProcessor::ProcessResult*>(result_ptr);
        res->mimoSoT_ = EMANE::TimePoint(std::chrono::microseconds(sot));
        res->mimoPropagationDelay_ = std::chrono::microseconds(prop_delay);
    }

    void emane_c_receive_processor_set_gain_cache_hit(void* result_ptr, bool hit) {
        auto res = static_cast<EMANE::ReceiveProcessor::ProcessResult*>(result_ptr);
        res->bGainCacheHit_ = hit;
    }

    void emane_c_receive_processor_add_doppler_shift(void* result_ptr, uint64_t freq, double shift) {
        auto res = static_cast<EMANE::ReceiveProcessor::ProcessResult*>(result_ptr);
        res->dopplerShifts_.insert(std::make_pair(freq, shift));
    }

    void emane_c_receive_processor_add_antenna_receive_info(
        void* result_ptr, uint16_t rx_ant, uint16_t tx_ant,
        uint64_t span, double rx_sensitivity_dbm,
        const void* freq_segments_ptr
    ) {
        auto res = static_cast<EMANE::ReceiveProcessor::ProcessResult*>(result_ptr);
        auto segs = static_cast<const EMANE::FrequencySegments*>(freq_segments_ptr);
        res->antennaReceiveInfos_.emplace_back(
            rx_ant, tx_ant, *segs, std::chrono::microseconds(span), rx_sensitivity_dbm
        );
    }
}
