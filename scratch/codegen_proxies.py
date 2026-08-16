import os

files = {
    "src/libemane/registrarproxy.cc": """#include "registrarproxy.h"

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
""",
    "src/libemane/registrarproxy.h": """#ifndef EMANEREGISTRARPROXY_HEADER_
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
""",
    "src/libemane/configurationregistrarproxy.h": """#ifndef EMANECONFIGURATIONREGISTRARPROXY_HEADER_
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
""",
    "src/libemane/configurationregistrarproxy.cc": """#include "configurationregistrarproxy.h"
#include "emane/registrarexception.h"
#include <cstring>

extern "C" {
    void emane_rs_config_register_numeric_any(uint16_t build_id, const char* name, int32_t type, uint64_t properties, const void* values_ptr, size_t values_len, const char* usage, const void* min_ptr, const void* max_ptr, size_t min_occurs, size_t max_occurs, const char* regex_pattern, char* err_buf, size_t err_len);
    void emane_rs_config_register_non_numeric_any(uint16_t build_id, const char* name, int32_t type, uint64_t properties, const void* values_ptr, size_t values_len, const char* usage, size_t min_occurs, size_t max_occurs, const char* regex_pattern, char* err_buf, size_t err_len);
    void emane_rs_config_register_validator(uint16_t build_id, void* validator, char* err_buf, size_t err_len);
    
    // We reuse the FfiAnyOwned struct from configuration.rs but map it via C++ here.
    struct CppFfiAny {
        int32_t any_type;
        int64_t i64_value;
        uint64_t u64_value;
        double d_value;
        const char* s_value;
    };
}

namespace {
    CppFfiAny makeFfiAny(const EMANE::Any& val) {
        CppFfiAny ffi{};
        ffi.any_type = static_cast<int32_t>(val.getType());
        if (ffi.any_type == static_cast<int32_t>(EMANE::Any::Type::TYPE_INT64)) ffi.i64_value = val.asINT64();
        else if (ffi.any_type == static_cast<int32_t>(EMANE::Any::Type::TYPE_UINT64)) ffi.u64_value = val.asUINT64();
        else if (ffi.any_type == static_cast<int32_t>(EMANE::Any::Type::TYPE_DOUBLE)) ffi.d_value = val.asDouble();
        else if (ffi.any_type == static_cast<int32_t>(EMANE::Any::Type::TYPE_STRING)) ffi.s_value = val.asString().c_str();
        return ffi;
    }
}

EMANE::ConfigurationRegistrarProxy::ConfigurationRegistrarProxy(BuildId buildId):
  buildId_{buildId}{}

void EMANE::ConfigurationRegistrarProxy::registerNumericAny(const std::string & sName,
                                                            Any::Type type,
                                                            const ConfigurationProperties & properties,
                                                            const std::vector<Any> & values,
                                                            const std::string & sUsage,
                                                            const Any & minValue,
                                                            const Any & maxValue,
                                                            std::size_t minOccurs,
                                                            std::size_t maxOccurs,
                                                            const std::string & sRegexPattern)
{
    std::vector<CppFfiAny> ffi_values;
    for (const auto& v : values) ffi_values.push_back(makeFfiAny(v));
    CppFfiAny min_ffi = makeFfiAny(minValue);
    CppFfiAny max_ffi = makeFfiAny(maxValue);
    
    char err_buf[256] = {0};
    emane_rs_config_register_numeric_any(buildId_, sName.c_str(), static_cast<int32_t>(type), static_cast<uint64_t>(properties), ffi_values.empty() ? nullptr : ffi_values.data(), ffi_values.size(), sUsage.c_str(), &min_ffi, &max_ffi, minOccurs, maxOccurs, sRegexPattern.c_str(), err_buf, sizeof(err_buf));
    if (err_buf[0] != '\\0') {
        throw makeException<RegistrarException>("%s", err_buf);
    }
}

void EMANE::ConfigurationRegistrarProxy::registerNonNumericAny(const std::string & sName,
                                                               Any::Type type,
                                                               const ConfigurationProperties & properties,
                                                               const std::vector<Any> & values,
                                                               const std::string & sUsage,
                                                               std::size_t minOccurs,
                                                               std::size_t maxOccurs,
                                                               const std::string & sRegexPattern)
{
    std::vector<CppFfiAny> ffi_values;
    for (const auto& v : values) ffi_values.push_back(makeFfiAny(v));
    
    char err_buf[256] = {0};
    emane_rs_config_register_non_numeric_any(buildId_, sName.c_str(), static_cast<int32_t>(type), static_cast<uint64_t>(properties), ffi_values.empty() ? nullptr : ffi_values.data(), ffi_values.size(), sUsage.c_str(), minOccurs, maxOccurs, sRegexPattern.c_str(), err_buf, sizeof(err_buf));
    if (err_buf[0] != '\\0') {
        throw makeException<RegistrarException>("%s", err_buf);
    }
}

void EMANE::ConfigurationRegistrarProxy::registerValidator(ConfigurationValidator validator)
{
    char err_buf[256] = {0};
    auto pValidator = new ConfigurationValidator(validator);
    emane_rs_config_register_validator(buildId_, pValidator, err_buf, sizeof(err_buf));
    if (err_buf[0] != '\\0') {
        throw makeException<RegistrarException>("%s", err_buf);
    }
}
""",
    "src/libemane/statisticregistrarproxy.h": """#ifndef EMANESTATISTICREGISTRARPROXY_HEADER_
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
    
    void registerTable(const std::string & sName,
                       const StatisticProperties & properties,
                       const std::string & sDescription,
                       StatisticTablePublisher * pStatisticTablePublisher,
                       std::function<void(StatisticTablePublisher *)> clearFunc) override;

  private:
    BuildId buildId_;
  };
}

#endif // EMANESTATISTICREGISTRARPROXY_HEADER_
""",
    "src/libemane/statisticregistrarproxy.cc": """#include "statisticregistrarproxy.h"
#include "emane/registrarexception.h"

extern "C" {
    void emane_rs_statistic_register(uint16_t build_id, const char* name, int32_t type, uint64_t properties, const char* desc, void* p_statistic, char* err_buf, size_t err_len);
    void emane_rs_statistic_register_table(uint16_t build_id, const char* name, uint64_t properties, const char* desc, void* p_table, void* p_clear_func, char* err_buf, size_t err_len);
}

EMANE::StatisticRegistrarProxy::StatisticRegistrarProxy(BuildId buildId):
  buildId_{buildId}{}

void EMANE::StatisticRegistrarProxy::registerStatistic(const std::string & sName,
                                                       Any::Type type,
                                                       const StatisticProperties & properties,
                                                       const std::string & sDescription,
                                                       Statistic * pStatistic)
{
    char err_buf[256] = {0};
    emane_rs_statistic_register(buildId_, sName.c_str(), static_cast<int32_t>(type), static_cast<uint64_t>(properties), sDescription.c_str(), pStatistic, err_buf, sizeof(err_buf));
    if (err_buf[0] != '\\0') {
        throw makeException<RegistrarException>("%s", err_buf);
    }
}

void EMANE::StatisticRegistrarProxy::registerTable(const std::string & sName,
                                                   const StatisticProperties & properties,
                                                   const std::string & sDescription,
                                                   StatisticTablePublisher * pStatisticTablePublisher,
                                                   std::function<void(StatisticTablePublisher *)> clearFunc)
{
    char err_buf[256] = {0};
    auto pFunc = new std::function<void(StatisticTablePublisher *)>(clearFunc);
    emane_rs_statistic_register_table(buildId_, sName.c_str(), static_cast<uint64_t>(properties), sDescription.c_str(), pStatisticTablePublisher, pFunc, err_buf, sizeof(err_buf));
    if (err_buf[0] != '\\0') {
        throw makeException<RegistrarException>("%s", err_buf);
    }
}
""",
    "src/libemane/eventregistrarproxy.h": """#ifndef EMANEEVENTREGISTRARPROXY_HEADER_
#define EMANEEVENTREGISTRARPROXY_HEADER_

#include "emane/eventregistrar.h"
#include "emane/types.h"

namespace EMANE
{
  class EventRegistrarProxy : public EventRegistrar
  {
  public:
    EventRegistrarProxy(BuildId buildId);

    void registerEvent(EventId eventId) override;

  private:
    BuildId buildId_;
  };
}

#endif //EMANEEVENTREGISTRARPROXY_HEADER_
""",
    "src/libemane/eventregistrarproxy.cc": """#include "eventregistrarproxy.h"
#include "emane/registrarexception.h"

extern "C" {
    bool emane_rs_event_service_register_event(uint16_t build_id, uint16_t event_id);
}

EMANE::EventRegistrarProxy::EventRegistrarProxy(BuildId buildId):
  buildId_{buildId}{}

void EMANE::EventRegistrarProxy::registerEvent(EventId eventId)
{
    if (!emane_rs_event_service_register_event(buildId_, eventId)) {
        throw RegistrarException{"Component not eligible to register for events"};
    }
}
"""
}

for path, content in files.items():
    with open(f"/home/joe/src/sleepybishop/emane/{path}", "w") as f:
        f.write(content)

print("Rewrote proxy classes to bypass Singletons.")
