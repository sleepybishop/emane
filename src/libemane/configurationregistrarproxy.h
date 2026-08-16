#ifndef EMANECONFIGURATIONREGISTRARPROXY_HEADER_
#define EMANECONFIGURATIONREGISTRARPROXY_HEADER_

#include "emane/configurationregistrar.h"
#include "emane/types.h"

namespace EMANE
{
  class ConfigurationRegistrarProxy : public ConfigurationRegistrar
  {
  public:
    ConfigurationRegistrarProxy(BuildId buildId);

    void registerNumericAny(const std::string & sName,
                            Any::Type type,
                            const ConfigurationProperties & properties,
                            const std::vector<Any> & values,
                            const std::string & sUsage,
                            const Any & minValue,
                            const Any & maxValue,
                            std::size_t minOccurs,
                            std::size_t maxOccurs,
                            const std::string & sRegexPattern) override;

    void registerNonNumericAny(const std::string & sName,
                               Any::Type type,
                               const ConfigurationProperties & properties,
                               const std::vector<Any> & values,
                               const std::string & sUsage,
                               std::size_t minOccurs,
                               std::size_t maxOccurs,
                               const std::string & sRegexPattern) override;

    void registerValidator(ConfigurationValidator validator) override;

  private:
    BuildId buildId_;
  };
}

#endif // EMANECONFIGURATIONREGISTRARPROXY_HEADER_
