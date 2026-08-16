#ifndef EMANESTATISTICREGISTRARPROXY_HEADER_
#define EMANESTATISTICREGISTRARPROXY_HEADER_

#include "emane/statisticregistrar.h"
#include "emane/types.h"

namespace EMANE
{
  class StatisticRegistrarProxy : public StatisticRegistrar
  {
  public:
    StatisticRegistrarProxy(BuildId buildId);

    void registerStatistic(const std::string & sName,
                           Any::Type type,
                           const StatisticProperties & properties,
                           const std::string & sDescription,
                           Statistic * pStatistic) override;
    
    void registerTablePublisher(const std::string & sName,
                       const StatisticProperties & properties,
                       const std::string & sDescription,
                       StatisticTablePublisher * pStatisticTablePublisher,
                       std::function<void(StatisticTablePublisher *)> clearFunc) override;

  private:
    BuildId buildId_;
  };
}

#endif // EMANESTATISTICREGISTRARPROXY_HEADER_
