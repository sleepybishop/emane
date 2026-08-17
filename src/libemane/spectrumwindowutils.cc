/*
 * Copyright (c) 2013,2020 - Adjacent Link LLC, Bridgewater, New Jersey
 * All rights reserved.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions
 * are met:
 *
 * * Redistributions of source code must retain the above copyright
 *   notice, this list of conditions and the following disclaimer.
 * * Redistributions in binary form must reproduce the above copyright
 *   notice, this list of conditions and the following disclaimer in
 *   the documentation and/or other materials provided with the
 *   distribution.
 * * Neither the name of Adjacent Link LLC nor the names of its
 *   contributors may be used to endorse or promote products derived
 *   from this software without specific prior written permission.
 *
 * THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS
 * "AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT
 * LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS
 * FOR A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE
 * COPYRIGHT OWNER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT,
 * INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING,
 * BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES;
 * LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER
 * CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT
 * LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN
 * ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE
 * POSSIBILITY OF SUCH DAMAGE.
 */

#include "emane/utils/spectrumwindowutils.h"
#include "emane/utils/conversionutils.h"
#include "emane/spectrumserviceexception.h"
#include <algorithm>

extern "C" {
    struct EmaneRsSpectrumCompressedEntry {
        size_t index;
        double value;
    };

    struct EmaneRsSpectrumCompressedRepresentation {
        EmaneRsSpectrumCompressedEntry* ptr;
        size_t len;
        size_t cap;
    };

    struct EmaneRsSpectrumSubBandCompressedEntry {
        size_t index;
        double* ptr;
        size_t len;
        size_t cap;
    };

    struct EmaneRsSpectrumSubBandCompressedRepresentation {
        EmaneRsSpectrumSubBandCompressedEntry* ptr;
        size_t len;
        size_t cap;
    };

    struct EmaneRsMaxBinNoiseFloorResult {
        double noise_floor_db;
        bool is_signal_in_noise;
        int error_code;
    };

    EmaneRsSpectrumCompressedRepresentation emane_rs_spectrum_compress(const double* window_ptr, size_t window_len);
    void emane_rs_spectrum_compress_free(EmaneRsSpectrumCompressedRepresentation repr);

    EmaneRsSpectrumSubBandCompressedRepresentation emane_rs_spectrum_sub_band_compress(const double* window_ptr, size_t window_len, size_t sub_band_bin_count);
    void emane_rs_spectrum_sub_band_compress_free(EmaneRsSpectrumSubBandCompressedRepresentation repr);

    EmaneRsMaxBinNoiseFloorResult emane_rs_max_bin_noise_floor(
        const double* noise_data_ptr,
        size_t noise_data_len,
        double rx_sensitivity_milliwatt,
        double rx_power_dbm,
        bool b_signal_in_noise,
        size_t start_bin,
        size_t end_bin);
}

std::pair<double,bool> EMANE::Utils::maxBinNoiseFloorRange(const SpectrumWindow & window,
                                                           double dRxPowerdBm,
                                                           const TimePoint & startTime,
                                                           const TimePoint & endTime)
{
  const auto & noiseData = std::get<0>(window);
  const TimePoint & windowStartTime = std::get<1>(window);
  const Microseconds & binSize =  std::get<2>(window);
  const double & dRxSensitivityMilliWatt = std::get<3>(window);
  const bool & bSignalInNoise{std::get<4>(window)};

  Microseconds::rep windowStartBin{timepointToAbsoluteBin(windowStartTime,binSize,false)};

  std::size_t startIndex{};
  std::size_t endIndex{noiseData.size()-1};

  if(startTime < windowStartTime)
    {
      throw makeException<SpectrumServiceException>("max bin start time < window start time");
    }
  else
    {
      startIndex = timepointToAbsoluteBin(startTime,binSize,false) - windowStartBin;
    }

  if(endTime != TimePoint::min())
    {
      if(endTime < startTime)
        {
          throw makeException<SpectrumServiceException>("max bin end time < max bin start time");
        }
      else
        {
          endIndex =  timepointToAbsoluteBin(endTime,binSize,true) - windowStartBin;
        }
    }

  return maxBinNoiseFloor(noiseData,dRxSensitivityMilliWatt,dRxPowerdBm,bSignalInNoise,startIndex,endIndex);
}

std::pair<double,bool> EMANE::Utils::maxBinNoiseFloor(const SpectrumWindow & window,
                                                      double dRxPowerdBm,
                                                      const TimePoint & startTime)
{
  const auto & noiseData = std::get<0>(window);
  const TimePoint & windowStartTime = std::get<1>(window);
  const Microseconds & binSize =  std::get<2>(window);
  const double & dRxSensitivityMilliWatt = std::get<3>(window);
  const bool & bSignalInNoise{std::get<4>(window)};

  Microseconds::rep windowStartBin{timepointToAbsoluteBin(windowStartTime,binSize,false)};

  std::size_t startIndex{};
  std::size_t endIndex{noiseData.size()-1};

  if(startTime != TimePoint::min())
    {
      if(startTime < windowStartTime)
        {
          throw makeException<SpectrumServiceException>("max bin start time %13.6f < window start time %13.6f",
                                                        std::chrono::duration_cast<DoubleSeconds>(startTime.time_since_epoch()).count(),
                                                        std::chrono::duration_cast<DoubleSeconds>(windowStartTime.time_since_epoch()).count());
        }
      else
        {
          startIndex = timepointToAbsoluteBin(startTime,binSize,false) - windowStartBin;
        }
    }

  return maxBinNoiseFloor(noiseData,dRxSensitivityMilliWatt,dRxPowerdBm,bSignalInNoise,startIndex,endIndex);
}

std::pair<double,bool> EMANE::Utils::maxBinNoiseFloor(const std::vector<double> & noiseData,
                                                      double dRxSensitivityMilliWatt,
                                                      double dRxPowerdBm,
                                                      bool bSignalInNoise,
                                                      std::size_t startBin,
                                                      std::size_t endBin)
{
  auto result = emane_rs_max_bin_noise_floor(
      noiseData.data(),
      noiseData.size(),
      dRxSensitivityMilliWatt,
      dRxPowerdBm,
      bSignalInNoise,
      startBin,
      endBin);

  if (result.error_code == 1)
    {
      throw makeException<SpectrumServiceException>("max bin end index %zu < max bin start index %zu,"
                                                    " num bins %zu",
                                                    startBin,
                                                    endBin,
                                                    noiseData.size());
    }
  else if (result.error_code == 2)
    {
      throw makeException<SpectrumServiceException>("bin index out of range, start index %zu,"
                                                    " end index %zu, num bins %zu",
                                                    startBin,
                                                    endBin,
                                                    noiseData.size());
    }

  return {result.noise_floor_db, result.is_signal_in_noise};
}

EMANE::Microseconds::rep EMANE::Utils::timepointToAbsoluteBin(const TimePoint & tp,
                                                              const Microseconds & binSize,
                                                              bool bAdjust)
{
  auto count =
    std::chrono::duration_cast<Microseconds>(tp.time_since_epoch()).count();

  return count == 0 ? 0 : count / binSize.count() - (bAdjust && (count % binSize.count() == 0));
}

EMANE::Utils::SpectrumCompressedRepresentation
EMANE::Utils::spectrumCompress(const std::vector<double> & window)
{
  SpectrumCompressedRepresentation ret;

  auto repr = emane_rs_spectrum_compress(window.data(), window.size());

  for (size_t i = 0; i < repr.len; ++i)
    {
      ret.push_back(std::make_pair(repr.ptr[i].index, repr.ptr[i].value));
    }

  emane_rs_spectrum_compress_free(repr);

  return ret;
}

EMANE::Utils::SpectrumSubBandCompressedRepresentation
EMANE::Utils::spectrumSubBandCompress(const std::vector<double> & window,
                                      std::size_t subBandBinCount)
{
  SpectrumSubBandCompressedRepresentation ret;

  auto repr = emane_rs_spectrum_sub_band_compress(window.data(), window.size(), subBandBinCount);

  for (size_t i = 0; i < repr.len; ++i)
    {
      std::vector<double> subBandVals(repr.ptr[i].ptr, repr.ptr[i].ptr + repr.ptr[i].len);
      ret.push_back(std::make_pair(repr.ptr[i].index, subBandVals));
    }

  emane_rs_spectrum_sub_band_compress_free(repr);

  return ret;
}
