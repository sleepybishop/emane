#include "registrarproxy.h"

EMANE::RegistrarProxy::RegistrarProxy(BuildId buildId):
  configurationRegistrarProxy_{buildId},
  statisticRegistrarProxy_{buildId},
  eventRegistrarProxy_{buildId}{}


EMANE::ConfigurationRegistrar & EMANE::RegistrarProxy::configurationRegistrar()
{
  return configurationRegistrarProxy_;
}

EMANE::StatisticRegistrar & EMANE::RegistrarProxy::statisticRegistrar()
{
  return statisticRegistrarProxy_;
}

EMANE::EventRegistrar & EMANE::RegistrarProxy::eventRegistrar()
{
  return eventRegistrarProxy_;
}
