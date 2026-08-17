#ifndef EMANENAKAGAMIFADINGALGORITHM_HEADER_
#define EMANENAKAGAMIFADINGALGORITHM_HEADER_

#include "fadingalgorithm.h"
#include "emane/utils/conversionutils.h"

namespace EMANE
{
  class NakagamiFadingAlgorithm: public FadingAlgorithm
  {
  public:
    NakagamiFadingAlgorithm(NEMId id,
                            PlatformServiceProvider * pPlatformService);

    ~NakagamiFadingAlgorithm();

    struct Parameters
    {
      double dDistance0Meters_{};
      double dDistance1Meters_{};
      double dm0_{};
      double dm1_{};
      double dm2_{};
    };

    double operator()(double dPowerdBm, double dDistanceMeters, const void * pParams) override;

  private:
    void * pState_;
  };
}

#endif // EMANENAKAGAMIFADINGALGORITHM_HEADER_
