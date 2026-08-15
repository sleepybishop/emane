/*
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that:
 *
 * (1) source code distributions retain this paragraph in its entirety,
 *
 * (2) distributions including binary code include this paragraph in
 *     its entirety in the documentation or other materials provided
 *     with the distribution.
 *
 *      "This product includes software written and developed
 *       by Code 5520 of the Naval Research Laboratory (NRL)."
 *
 *  The name of NRL, the name(s) of NRL  employee(s), or any entity
 *  of the United States Government may not be used to endorse or
 *  promote  products derived from this software, nor does the
 *  inclusion of the NRL written and developed software  directly or
 *  indirectly suggest NRL or United States  Government endorsement
 *  of this product.
 *
 * THIS SOFTWARE IS PROVIDED "AS IS" AND WITHOUT ANY EXPRESS OR
 * IMPLIED WARRANTIES, INCLUDING, WITHOUT LIMITATION, THE IMPLIED
 * WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE.
 */

#ifndef EMANELOGNORMALFADINGALGORITHM_HEADER_
#define EMANELOGNORMALFADINGALGORITHM_HEADER_

#include "fadingalgorithm.h"
#include "emane/utils/conversionutils.h"
#include <random>

extern "C" {
  struct LognormalFadingState;
  
  struct LognormalFadingParameters {
    double dmu;
    double dsigma;
    double dlthresh;
    double maxpathloss;
    double duthresh;
    double minpathloss;
    double lmean;
    double lstddev;
    unsigned int counter;
  };

  LognormalFadingState* emane_rs_lognormal_fading_new();
  void emane_rs_lognormal_fading_free(LognormalFadingState* state);
  double emane_rs_lognormal_fading_process(LognormalFadingState* state, double power_dbm, const LognormalFadingParameters* params, uint64_t now_microsec);
}

namespace EMANE
{
  class LognormalFadingAlgorithm: public FadingAlgorithm
  {
  public:
    LognormalFadingAlgorithm(NEMId id,
                            PlatformServiceProvider * pPlatformService);

    ~LognormalFadingAlgorithm();

    struct Parameters
    {
      double dmu_{};
      double dsigma_{};
      double dlthresh_{};
      double maxpathloss_{};
      double duthresh_{};
      double minpathloss_{};
      double lmean_{};
      double lstddev_{};
      unsigned int counter_{};
    };

    double operator()(double dPowerdBm, double, const void * pParams) override
    {
      auto pLognormalFadingParameters = reinterpret_cast<const LognormalFadingParameters *>(pParams);

      TimePoint now=Clock::now();
      auto now_microsec = std::chrono::time_point_cast<std::chrono::microseconds>(now).time_since_epoch().count();

      return emane_rs_lognormal_fading_process(pState_, dPowerdBm, pLognormalFadingParameters, now_microsec);
    }

  private:
    LognormalFadingState* pState_;
  };
}

#endif // EMANELOGNORMALFADINGALGORITHM_HEADER_
