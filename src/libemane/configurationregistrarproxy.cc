#include "configurationregistrarproxy.h"
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
    if (err_buf[0] != '\0') {
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
    if (err_buf[0] != '\0') {
        throw makeException<RegistrarException>("%s", err_buf);
    }
}

void EMANE::ConfigurationRegistrarProxy::registerValidator(ConfigurationValidator validator)
{
    char err_buf[256] = {0};
    auto pValidator = new ConfigurationValidator(validator);
    emane_rs_config_register_validator(buildId_, pValidator, err_buf, sizeof(err_buf));
    if (err_buf[0] != '\0') {
        throw makeException<RegistrarException>("%s", err_buf);
    }
}
