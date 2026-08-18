#ifndef EMANEPHYSPECTRUMMONITOR_HEADER_
#define EMANEPHYSPECTRUMMONITOR_HEADER_

#include "emane/types.h"
#include "emane/frequencysegment.h"
#include "emane/spectrumserviceprovider.h"
#include "emane/filtermatchcriterion.h"
#include "noisemode.h"
#include "spectralmaskmanager.h"

#include <vector>
#include <tuple>
#include <mutex>

namespace EMANE
{
  using SpectrumUpdate =
    std::tuple<TimePoint,Microseconds,Microseconds,FrequencySegments,bool,double>;

  class SpectrumMonitor
  {
  public:
    SpectrumMonitor();
    ~SpectrumMonitor();

    void initialize(uint16_t u16SubId,
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
                    bool bExcludeSameSubIdFromFilter);

    SpectrumUpdate
    update(const TimePoint & now,
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
           const std::pair<FilterData,bool> & optionalFilterData);


    FrequencySet getFrequencies() const;

    double getReceiverSensitivitydBm() const;

    // test harness access
    SpectrumWindow request_i(const TimePoint & now,
                             std::uint64_t u64FrequencyHz,
                             const Microseconds & duration = Microseconds::zero(),
                             const TimePoint & timepoint = TimePoint::min()) const;


    SpectrumWindow request(std::uint64_t u64FrequencyHz,
                           const Microseconds & duration = Microseconds::zero(),
                           const TimePoint & timepoint = TimePoint::min()) const;

    void initializeFilter(FilterIndex filterIndex,
                          std::uint64_t u64FrequencyHz,
                          std::uint64_t u64BandwidthHz,
                          std::uint64_t u64BandwidthBinSizeHz,
                          const FilterMatchCriterion * pFilterMatchCriterion);

    void removeFilter(FilterIndex filterIndex);

    // test harness access
    SpectrumFilterWindow requestFilter_i(const TimePoint & now,
                                         FilterIndex filterIndex,
                                         const Microseconds & duration = Microseconds::zero(),
                                         const TimePoint & timepoint = TimePoint::min()) const;

    SpectrumFilterWindow requestFilter(FilterIndex filterIndex,
                                       const Microseconds & duration = Microseconds::zero(),
                                       const TimePoint & timepoint = TimePoint::min()) const;

    std::vector<double> dump(std::uint64_t u64FrequencyHz) const;

    std::pair<std::vector<double>,std::size_t> dumpFilter(FilterIndex filterIndex) const;

  private:
    void* pRustMonitor_;
  };
}

#endif // EMANEPHYSPECTRUMMONITOR_HEADER_
