#include "noiserecorder.h"
#include "frequencyoverlapratio.h"
#include "emane/spectrumserviceexception.h"
#include <cmath>
#include <iostream>

extern "C" {
    void* emane_rs_noise_recorder_new(
        int64_t bin,
        int64_t max_offset,
        int64_t max_propagation,
        int64_t max_duration,
        double d_rx_sensitivity_milli_watt,
        uint64_t u64_frequency_hz,
        uint64_t u64_bandwidth_hz,
        uint64_t u64_bandwidth_bin_size_hz
    );

    void emane_rs_noise_recorder_free(void* ptr);

    void emane_rs_noise_recorder_update(
        void* ptr,
        int64_t now,
        int64_t tx_time,
        int64_t offset,
        int64_t propagation,
        int64_t duration,
        double d_rx_power,
        const uint16_t* transmitters_ptr,
        size_t transmitters_len,
        uint64_t u64_start_frequency_hz,
        uint64_t u64_end_frequency_hz,
        uint16_t tx_antenna_index,
        bool b_is_more,
        int64_t* out_sor,
        int64_t* out_eor
    );

    struct EmaneRsNoiseWindowResult {
        double* data;
        size_t length;
        int64_t start_of_window_time;
    };

    EmaneRsNoiseWindowResult emane_rs_noise_recorder_get(
        const void* ptr,
        int64_t now,
        int64_t duration,
        int64_t start_time
    );

    void emane_rs_noise_recorder_free_window(double* ptr, size_t len);

    size_t emane_rs_noise_recorder_get_sub_band_bin_count(const void* ptr);

    double* emane_rs_noise_recorder_dump(const void* ptr, size_t* out_len);
}

EMANE::NoiseRecorder::NoiseRecorder(const Microseconds & bin,
                                    const Microseconds & maxOffset,
                                    const Microseconds & maxPropagation,
                                    const Microseconds & maxDuration,
                                    double dRxSensitivityMilliWatt,
                                    std::uint64_t u64FrequencyHz,
                                    std::uint64_t u64BandwidthHz,
                                    std::uint64_t u64BandwidthBinSizeHz):
  totalWindowBins_{maxDuration/bin},
  totalWheelBins_{(maxOffset + maxPropagation + 2 * maxDuration)/bin},
  binSizeMicroseconds_{bin.count()},
  u64BandwidthBinSizeHz_{u64BandwidthBinSizeHz},
  u64BandStartFrequencyHz_{static_cast<std::uint64_t>(u64FrequencyHz - u64BandwidthHz / 2.0)},
  totalSubBandBins_{u64BandwidthBinSizeHz ? static_cast<size_t>(std::ceil(u64BandwidthHz/static_cast<double>(u64BandwidthBinSizeHz)))+1 : 1},
  u64BandEndFrequencyHz_{u64BandStartFrequencyHz_ +  totalSubBandBins_ * u64BandwidthBinSizeHz - 1},
  wheel_{static_cast<std::size_t>(totalWheelBins_), totalSubBandBins_},
  dRxSensitivityMilliWatt_{dRxSensitivityMilliWatt},
  maxEndOfReceptionBin_{},
  minStartOfReceptionBin_{}
{
    rs_recorder_ = emane_rs_noise_recorder_new(
        bin.count(),
        maxOffset.count(),
        maxPropagation.count(),
        maxDuration.count(),
        dRxSensitivityMilliWatt,
        u64FrequencyHz,
        u64BandwidthHz,
        u64BandwidthBinSizeHz
    );
}

EMANE::NoiseRecorder::~NoiseRecorder()
{
    if (rs_recorder_) {
        emane_rs_noise_recorder_free(rs_recorder_);
        rs_recorder_ = nullptr;
    }
}

std::tuple<EMANE::TimePoint,EMANE::TimePoint>
EMANE::NoiseRecorder::update(const TimePoint & now,
                             const TimePoint & txTime,
                             const Microseconds & offset,
                             const Microseconds & propagation,
                             const Microseconds & duration,
                             double dRxPower,
                             const std::vector<NEMId> & transmitters,
                             std::uint64_t u64StartFrequencyHz,
                             std::uint64_t u64EndFrequencyHz,
                             AntennaIndex txAntennaIndex,
                             bool bIsMore)
{
    int64_t out_sor = 0;
    int64_t out_eor = 0;
    
    emane_rs_noise_recorder_update(
        rs_recorder_,
        std::chrono::duration_cast<std::chrono::microseconds>(now.time_since_epoch()).count(),
        std::chrono::duration_cast<std::chrono::microseconds>(txTime.time_since_epoch()).count(),
        offset.count(),
        propagation.count(),
        duration.count(),
        dRxPower,
        transmitters.data(),
        transmitters.size(),
        u64StartFrequencyHz,
        u64EndFrequencyHz,
        txAntennaIndex,
        bIsMore,
        &out_sor,
        &out_eor
    );

    return std::make_tuple(
        TimePoint(std::chrono::microseconds(out_sor)),
        TimePoint(std::chrono::microseconds(out_eor))
    );
}

std::pair<std::vector<double>, EMANE::TimePoint>
EMANE::NoiseRecorder::get(const TimePoint & now,
                          const Microseconds & duration,
                          const TimePoint & startTime)
{
    int64_t now_micros = std::chrono::duration_cast<std::chrono::microseconds>(now.time_since_epoch()).count();
    int64_t start_time_micros = (startTime == TimePoint::min()) ? -9223372036854775807LL - 1 : std::chrono::duration_cast<std::chrono::microseconds>(startTime.time_since_epoch()).count();
    
    EmaneRsNoiseWindowResult result = emane_rs_noise_recorder_get(
        rs_recorder_,
        now_micros,
        duration.count(),
        start_time_micros
    );

    std::vector<double> window(result.data, result.data + result.length);
    emane_rs_noise_recorder_free_window(result.data, result.length);

    return std::make_pair(window, TimePoint(std::chrono::microseconds(result.start_of_window_time)));
}

std::vector<double> EMANE::NoiseRecorder::dump() const
{
    size_t len = 0;
    double* data = emane_rs_noise_recorder_dump(rs_recorder_, &len);
    if (!data) return {};
    std::vector<double> result(data, data + len);
    emane_rs_noise_recorder_free_window(data, len);
    return result;
}

EMANE::Microseconds::rep EMANE::NoiseRecorder::timepointToBin(const TimePoint & tp,bool bAdjust)
{
  auto count =
    std::chrono::duration_cast<Microseconds>(tp.time_since_epoch()).count();
  return count == 0 ? 0 : count / binSizeMicroseconds_ - (bAdjust && (count % binSizeMicroseconds_ == 0));
}

std::size_t EMANE::NoiseRecorder::getSubBandBinCount() const
{
  return emane_rs_noise_recorder_get_sub_band_bin_count(rs_recorder_);
}
