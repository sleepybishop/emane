#ifndef EMANEREGISTRARPROXY_HEADER_
#define EMANEREGISTRARPROXY_HEADER_

#include "emane/registrar.h"
#include "configurationregistrarproxy.h"
#include "statisticregistrarproxy.h"
#include "eventregistrarproxy.h"

namespace EMANE
{
  class RegistrarProxy : public Registrar
  {
  public:
    RegistrarProxy(BuildId buildId);

    ConfigurationRegistrar & configurationRegistrar() override;

    StatisticRegistrar & statisticRegistrar() override;

    EventRegistrar & eventRegistrar() override;
    
  private:
    ConfigurationRegistrarProxy configurationRegistrarProxy_;
    StatisticRegistrarProxy statisticRegistrarProxy_;
    EventRegistrarProxy eventRegistrarProxy_;
  };
}

#endif // EMANEREGISTRARPROXY_HEADER_
