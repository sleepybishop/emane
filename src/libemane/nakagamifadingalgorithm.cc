#include <cstdint>
#include "nakagamifadingalgorithm.h"

extern "C" {
    void* emane_rs_nakagami_fading_new();
    void emane_rs_nakagami_fading_free(void* ptr);
    double emane_rs_nakagami_fading_compute(void* ptr, double power_dbm, double distance_meters, double d0, double d1, double m0, double m1, double m2);
}

EMANE::NakagamiFadingAlgorithm::NakagamiFadingAlgorithm(NEMId id,
                                                        PlatformServiceProvider * pPlatformService):
  FadingAlgorithm{id,pPlatformService},
  pState_{emane_rs_nakagami_fading_new()}{}


EMANE::NakagamiFadingAlgorithm::~NakagamiFadingAlgorithm()
{
  emane_rs_nakagami_fading_free(pState_);
}

double EMANE::NakagamiFadingAlgorithm::operator()(double dPowerdBm, double dDistanceMeters, const void * pParams)
{
  auto pNakagamiFadingParameters = reinterpret_cast<const Parameters *>(pParams);
  
  return emane_rs_nakagami_fading_compute(pState_,
                                          dPowerdBm,
                                          dDistanceMeters,
                                          pNakagamiFadingParameters->dDistance0Meters_,
                                          pNakagamiFadingParameters->dDistance1Meters_,
                                          pNakagamiFadingParameters->dm0_,
                                          pNakagamiFadingParameters->dm1_,
                                          pNakagamiFadingParameters->dm2_);
}
