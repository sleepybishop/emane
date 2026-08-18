#include "spectrummonitor.h"
#include "noiserecorder.h"
#include "spectralmaskmanager.h"

#include "emane/spectrumserviceexception.h"
#include "emane/utils/conversionutils.h"
#include "emane/utils/dopplerutils.h"

#include <algorithm>
#include <functional>

extern "C" {
  void* emane_rs_spectrum_monitor_new();
  void emane_rs_spectrum_monitor_free(void* ptr);

  void emane_rs_spectrum_monitor_initialize(
      void* ptr,
      uint16_t u16SubId,
      const uint64_t* foi_ptr,
      size_t foi_len,
      uint64_t u64BandwidthHz,
      double dReceiverSensitivityMilliWatt,
      uint32_t mode,
      int64_t binSize,
      int64_t maxOffset,
      int64_t maxPropagation,
      int64_t maxDuration,
      int64_t timeSyncThreshold,
      bool bMaxClamp,
      bool bExcludeSameSubIdFromFilter);

  struct FfiFrequencySegment {
      uint64_t frequency_hz;
      double rx_power_dbm;
      int64_t duration_microsec;
      int64_t offset_microsec;
  };

  struct FfiSpectrumUpdate {
      int64_t tx_time;
      int64_t propagation_delay;
      int64_t duration;
      FfiFrequencySegment* segments;
      size_t segments_len;
      bool report_as_in_band;
      double receiver_sensitivity_mw;
  };

  FfiSpectrumUpdate emane_rs_spectrum_monitor_update(
      void* ptr,
      int64_t now,
      int64_t txTime,
      int64_t propagationDelay,
      double dDopplerFactor,
      const FfiFrequencySegment* segments,
      size_t segments_len,
      uint64_t u64SegmentBandwidthHz,
      const double* rxPowersMilliWatt,
      size_t rxPowersMilliWatt_len,
      bool bInBand,
      const uint16_t* transmitters,
      size_t transmitters_len,
      uint16_t u16SubId,
      uint16_t txAntennaIndex,
      uint16_t spectralMaskIndex,
      const uint8_t* filterData_ptr,
      size_t filterData_len);

  void emane_rs_spectrum_monitor_free_update_segments(FfiFrequencySegment* segments, size_t len);

  size_t emane_rs_spectrum_monitor_get_frequencies(const void* ptr, uint64_t* out_freqs, size_t max_len);

  double emane_rs_spectrum_monitor_get_receiver_sensitivity_dbm(const void* ptr);

  struct FfiSpectrumWindow {
      double* data;
      size_t length;
      int64_t start_of_window_time;
      int64_t bin_size_microsec;
      double receiver_sensitivity_mw;
      bool is_noise_all;
  };

  FfiSpectrumWindow emane_rs_spectrum_monitor_request_i(
      const void* ptr,
      int64_t now,
      uint64_t u64FrequencyHz,
      int64_t duration,
      int64_t timepoint);

  void emane_rs_spectrum_monitor_free_window_data(double* data, size_t len);

  void emane_rs_spectrum_monitor_initialize_filter(
      void* ptr,
      uint16_t filterIndex,
      uint64_t u64FrequencyHz,
      uint64_t u64BandwidthHz,
      uint64_t u64BandwidthBinSizeHz,
      const void* pFilterMatchCriterion);

  void emane_rs_spectrum_monitor_remove_filter(void* ptr, uint16_t filterIndex);

  struct FfiSpectrumFilterWindow {
      double* data;
      size_t length;
      int64_t start_of_window_time;
      int64_t bin_size_microsec;
      double receiver_sensitivity_mw;
      size_t sub_band_bin_count;
  };

  FfiSpectrumFilterWindow emane_rs_spectrum_monitor_request_filter_i(
      const void* ptr,
      int64_t now,
      uint16_t filterIndex,
      int64_t duration,
      int64_t timepoint);

  void emane_rs_spectrum_monitor_dump(const void* ptr, uint64_t u64FrequencyHz, double** out_data, size_t* out_len);
  
  void emane_rs_spectrum_monitor_dump_filter(const void* ptr, uint16_t filterIndex, double** out_data, size_t* out_len, size_t* out_sub_band_bin_count);
  
  bool emane_rs_filter_match(
        const void* criterion,
        uint64_t freq,
        uint64_t bw,
        uint16_t subid,
        const uint8_t* filter_data_ptr,
        size_t filter_data_len
  ) {
      const EMANE::FilterMatchCriterion* pFilterMatchCriterion = 
          reinterpret_cast<const EMANE::FilterMatchCriterion*>(criterion);
      if(pFilterMatchCriterion) {
          EMANE::FilterElementValues vals;
          vals.u64FrequencyHz_ = freq;
          vals.u64BandwidthHz_ = bw;
          vals.u16SubId_ = subid;
          if (filter_data_ptr && filter_data_len > 0) {
              vals.filterData_ = std::string(reinterpret_cast<const char*>(filter_data_ptr), filter_data_len);
          }
          return (*pFilterMatchCriterion)(vals);
      }
      return false;
  }
}

EMANE::SpectrumMonitor::SpectrumMonitor() {
    pRustMonitor_ = emane_rs_spectrum_monitor_new();
}

EMANE::SpectrumMonitor::~SpectrumMonitor() {
    if (pRustMonitor_) {
        emane_rs_spectrum_monitor_free(pRustMonitor_);
    }
}

void EMANE::SpectrumMonitor::initialize(std::uint16_t u16SubId,
                                        const FrequencySet & foi,
                                        std::uint64_t u64BandwidthHz,
                                        double dReceiverSensitivityMilliWatt,
                                        NoiseMode mode,
                                        const Microseconds & binSize,
                                        const Microseconds & maxOffset,
                                        const Microseconds & maxPropagation,
                                        const Microseconds & maxDuration,
                                        const Microseconds & timeSyncThreshold,
                                        bool bMaxClamp,
                                        bool bExcludeSameSubIdFromFilter)
{
    std::vector<uint64_t> freqs(foi.begin(), foi.end());
    emane_rs_spectrum_monitor_initialize(
        pRustMonitor_,
        u16SubId,
        freqs.data(),
        freqs.size(),
        u64BandwidthHz,
        dReceiverSensitivityMilliWatt,
        static_cast<uint32_t>(mode),
        binSize.count(),
        maxOffset.count(),
        maxPropagation.count(),
        maxDuration.count(),
        timeSyncThreshold.count(),
        bMaxClamp,
        bExcludeSameSubIdFromFilter
    );
}

EMANE::SpectrumUpdate
EMANE::SpectrumMonitor::update(const TimePoint & now,
                               const TimePoint & txTime,
                               const Microseconds & propagationDelay,
                               double dDopplerFactor,
                               const FrequencySegments & segments,
                               std::uint64_t u64SegmentBandwidthHz,
                               const std::vector<double> & rxPowersMilliWatt,
                               bool bInBand,
                               const std::vector<NEMId> & transmitters,
                               std::uint16_t u16SubId,
                               AntennaIndex txAntennaIndex,
                               SpectralMaskIndex spectralMaskIndex,
                               const std::pair<FilterData,bool> & optionalFilterData)
{
    std::vector<FfiFrequencySegment> ffi_segments;
    ffi_segments.reserve(segments.size());
    for(const auto& s : segments) {
        ffi_segments.push_back({
            s.getFrequencyHz(),
            s.getRxPowerdBm(),
            s.getDuration().count(),
            s.getOffset().count()
        });
    }

    const uint8_t* filterDataPtr = nullptr;
    size_t filterDataLen = 0;
    if(optionalFilterData.second) {
        filterDataPtr = reinterpret_cast<const uint8_t*>(optionalFilterData.first.data());
        filterDataLen = optionalFilterData.first.size();
    }

    FfiSpectrumUpdate update_res = emane_rs_spectrum_monitor_update(
        pRustMonitor_,
        std::chrono::duration_cast<Microseconds>(now.time_since_epoch()).count(),
        std::chrono::duration_cast<Microseconds>(txTime.time_since_epoch()).count(),
        propagationDelay.count(),
        dDopplerFactor,
        ffi_segments.data(),
        ffi_segments.size(),
        u64SegmentBandwidthHz,
        rxPowersMilliWatt.data(),
        rxPowersMilliWatt.size(),
        bInBand,
        transmitters.data(),
        transmitters.size(),
        u16SubId,
        txAntennaIndex,
        spectralMaskIndex,
        filterDataPtr,
        filterDataLen
    );

    TimePoint resTxTime{Microseconds{update_res.tx_time}};
    Microseconds resProp{update_res.propagation_delay};
    Microseconds resDur{update_res.duration};
    
    FrequencySegments out_segments;
    for(size_t i = 0; i < update_res.segments_len; ++i) {
        out_segments.emplace_back(
            update_res.segments[i].frequency_hz,
            update_res.segments[i].rx_power_dbm,
            Microseconds{update_res.segments[i].duration_microsec},
            Microseconds{update_res.segments[i].offset_microsec}
        );
    }
    
    emane_rs_spectrum_monitor_free_update_segments(update_res.segments, update_res.segments_len);

    return std::make_tuple(
        resTxTime,
        resProp,
        resDur,
        out_segments,
        update_res.report_as_in_band,
        update_res.receiver_sensitivity_mw
    );
}

EMANE::FrequencySet EMANE::SpectrumMonitor::getFrequencies() const {
    size_t len = emane_rs_spectrum_monitor_get_frequencies(pRustMonitor_, nullptr, 0);
    std::vector<uint64_t> freqs(len);
    emane_rs_spectrum_monitor_get_frequencies(pRustMonitor_, freqs.data(), len);
    return FrequencySet(freqs.begin(), freqs.end());
}

double EMANE::SpectrumMonitor::getReceiverSensitivitydBm() const {
    return emane_rs_spectrum_monitor_get_receiver_sensitivity_dbm(pRustMonitor_);
}

EMANE::SpectrumWindow
EMANE::SpectrumMonitor::request_i(const TimePoint & now,
                                  std::uint64_t u64FrequencyHz,
                                  const Microseconds & duration,
                                  const TimePoint & timepoint) const
{
    int64_t t_now = std::chrono::duration_cast<Microseconds>(now.time_since_epoch()).count();
    int64_t t_point = -9223372036854775807LL - 1; // TIMEPOINT_MIN conceptually
    if (timepoint != TimePoint::min()) {
        t_point = std::chrono::duration_cast<Microseconds>(timepoint.time_since_epoch()).count();
    }

    FfiSpectrumWindow w = emane_rs_spectrum_monitor_request_i(
        pRustMonitor_,
        t_now,
        u64FrequencyHz,
        duration.count(),
        t_point
    );

    std::vector<double> v;
    if(w.data) {
        v.assign(w.data, w.data + w.length);
        emane_rs_spectrum_monitor_free_window_data(w.data, w.length);
    }

    return std::make_tuple(
        v,
        TimePoint{Microseconds{w.start_of_window_time}},
        Microseconds{w.bin_size_microsec},
        w.receiver_sensitivity_mw,
        w.is_noise_all
    );
}

EMANE::SpectrumWindow
EMANE::SpectrumMonitor::request(std::uint64_t u64FrequencyHz,
                                const Microseconds & duration,
                                const TimePoint & timepoint) const
{
    return request_i(Clock::now(), u64FrequencyHz, duration, timepoint);
}

void EMANE::SpectrumMonitor::initializeFilter(FilterIndex filterIndex,
                                              std::uint64_t u64FrequencyHz,
                                              std::uint64_t u64BandwidthHz,
                                              std::uint64_t u64BandwidthBinSizeHz,
                                              const FilterMatchCriterion * pFilterMatchCriterion)
{
    emane_rs_spectrum_monitor_initialize_filter(
        pRustMonitor_,
        filterIndex,
        u64FrequencyHz,
        u64BandwidthHz,
        u64BandwidthBinSizeHz,
        reinterpret_cast<const void*>(pFilterMatchCriterion)
    );
}

void EMANE::SpectrumMonitor::removeFilter(FilterIndex filterIndex)
{
    emane_rs_spectrum_monitor_remove_filter(pRustMonitor_, filterIndex);
}

EMANE::SpectrumFilterWindow
EMANE::SpectrumMonitor::requestFilter_i(const TimePoint & now,
                                        FilterIndex filterIndex,
                                        const Microseconds & duration,
                                        const TimePoint & timepoint) const
{
    int64_t t_now = std::chrono::duration_cast<Microseconds>(now.time_since_epoch()).count();
    int64_t t_point = -9223372036854775807LL - 1; // TIMEPOINT_MIN
    if (timepoint != TimePoint::min()) {
        t_point = std::chrono::duration_cast<Microseconds>(timepoint.time_since_epoch()).count();
    }

    FfiSpectrumFilterWindow w = emane_rs_spectrum_monitor_request_filter_i(
        pRustMonitor_,
        t_now,
        filterIndex,
        duration.count(),
        t_point
    );

    std::vector<double> v;
    if(w.data) {
        v.assign(w.data, w.data + w.length);
        emane_rs_spectrum_monitor_free_window_data(w.data, w.length);
    }

    return std::make_tuple(
        v,
        TimePoint{Microseconds{w.start_of_window_time}},
        Microseconds{w.bin_size_microsec},
        w.receiver_sensitivity_mw,
        w.sub_band_bin_count
    );
}

EMANE::SpectrumFilterWindow
EMANE::SpectrumMonitor::requestFilter(FilterIndex filterIndex,
                                      const Microseconds & duration,
                                      const TimePoint & timepoint) const
{
    return requestFilter_i(Clock::now(), filterIndex, duration, timepoint);
}

std::vector<double> EMANE::SpectrumMonitor::dump(std::uint64_t u64FrequencyHz) const
{
    double* data = nullptr;
    size_t len = 0;
    emane_rs_spectrum_monitor_dump(pRustMonitor_, u64FrequencyHz, &data, &len);

    std::vector<double> v;
    if(data) {
        v.assign(data, data + len);
        emane_rs_spectrum_monitor_free_window_data(data, len);
    }
    return v;
}

std::pair<std::vector<double>,std::size_t> EMANE::SpectrumMonitor::dumpFilter(FilterIndex filterIndex) const
{
    double* data = nullptr;
    size_t len = 0;
    size_t sub_band_count = 0;
    emane_rs_spectrum_monitor_dump_filter(pRustMonitor_, filterIndex, &data, &len, &sub_band_count);

    std::vector<double> v;
    if(data) {
        v.assign(data, data + len);
        emane_rs_spectrum_monitor_free_window_data(data, len);
    }
    return {v, sub_band_count};
}

