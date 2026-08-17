#include <cstdint>
#include "configurationservice.h"
#include "emane/registrarexception.h"
#include "emane/configurationexception.h"
#include <cstring>
#include <iostream>

extern "C" {
    void* emane_rs_regex_compile(const char* pattern, char* error_buf, size_t error_buf_len);
    void emane_rs_regex_free(void* regex);
    bool emane_rs_regex_match(void* regex, const char* value);
}

extern "C" {
    struct FfiAny {
        int32_t any_type;
        int64_t i64_value;
        uint64_t u64_value;
        double d_value;
        const char* s_value;
    };

    struct FfiAnyArray {
        const FfiAny* data;
        size_t len;
    };

    struct FfiConfigItemUpdate {
        const char* name;
        FfiAnyArray values;
    };

    struct FfiConfigUpdate {
        const FfiConfigItemUpdate* data;
        size_t len;
    };

    struct FfiStringArray {
        const char** data;
        size_t len;
    };

    struct FfiConfigUpdateReqItem {
        const char* name;
        FfiStringArray values;
    };

    struct FfiConfigUpdateReq {
        const FfiConfigUpdateReqItem* data;
        size_t len;
    };

    struct FfiConfigInfo {
        const char* name;
        int32_t any_type;
        uint64_t properties;
        FfiAnyArray values;
        const char* usage;
        bool has_min_max;
        FfiAny min_value;
        FfiAny max_value;
        size_t min_occurs;
        size_t max_occurs;
        const char* regex_pattern;
    };

    struct FfiConfigManifest {
        FfiConfigInfo* data;
        size_t len;
    };

    // Rust exports
    void emane_rs_config_register_running_state_mutable(uint16_t buildId, void* ptr);
    void emane_rs_config_register_numeric_any(uint16_t buildId, const char* sName, int32_t type, uint64_t properties, FfiAnyArray values, const char* sUsage, FfiAny minValue, FfiAny maxValue, size_t minOccurs, size_t maxOccurs, const char* sRegexPattern, char* error_buf, size_t error_buf_len);
    void emane_rs_config_register_non_numeric_any(uint16_t buildId, const char* sName, int32_t type, uint64_t properties, FfiAnyArray values, const char* sUsage, size_t minOccurs, size_t maxOccurs, const char* sRegexPattern, char* error_buf, size_t error_buf_len);
    FfiConfigManifest emane_rs_config_get_manifest(uint16_t buildId);
    void emane_rs_config_free_manifest(FfiConfigManifest manifest);
    FfiConfigUpdate emane_rs_config_query(uint16_t buildId, FfiStringArray names);
    void emane_rs_config_free_update(FfiConfigUpdate update);
    FfiConfigUpdate emane_rs_config_build_updates(uint16_t buildId, FfiConfigUpdateReq parameters, char* error_buf, size_t error_buf_len);
    bool emane_rs_config_update(uint16_t buildId, FfiConfigUpdate updates, char* error_buf, size_t error_buf_len);
    void emane_rs_config_register_validator(uint16_t buildId, void* validator);

    // Callbacks from Rust back to C++
    bool emane_c_config_call_validator(void* pValidator, const FfiConfigUpdate* update, char* error_buf, size_t error_buf_len);
    void emane_c_config_process_configuration(void* pRunningStateMutable, const FfiConfigUpdate* update);
}

// Helpers
static void convertAnyToFfi(const EMANE::Any& any, FfiAny& ffiAny, std::vector<std::string>& stringStorage) {
    ffiAny.any_type = static_cast<int32_t>(any.getType());
    ffiAny.i64_value = 0;
    ffiAny.u64_value = 0;
    ffiAny.d_value = 0.0;
    ffiAny.s_value = nullptr;

    switch(any.getType()) {
        case EMANE::Any::Type::TYPE_INT64:
        case EMANE::Any::Type::TYPE_INT32:
        case EMANE::Any::Type::TYPE_INT16:
        case EMANE::Any::Type::TYPE_INT8:
            ffiAny.i64_value = any.asINT64();
            break;
        case EMANE::Any::Type::TYPE_UINT64:
        case EMANE::Any::Type::TYPE_UINT32:
        case EMANE::Any::Type::TYPE_UINT16:
        case EMANE::Any::Type::TYPE_UINT8:
            ffiAny.u64_value = any.asUINT64();
            break;
        case EMANE::Any::Type::TYPE_FLOAT:
        case EMANE::Any::Type::TYPE_DOUBLE:
            ffiAny.d_value = any.asDouble();
            break;
        case EMANE::Any::Type::TYPE_BOOL:
            ffiAny.u64_value = any.asBool() ? 1 : 0;
            break;
        case EMANE::Any::Type::TYPE_INET_ADDR:
        case EMANE::Any::Type::TYPE_STRING:
            stringStorage.push_back(any.asString());
            ffiAny.s_value = stringStorage.back().c_str();
            break;
    }
}

static EMANE::Any convertFfiToAny(const FfiAny& ffiAny) {
    auto type = static_cast<EMANE::Any::Type>(ffiAny.any_type);
    switch(type) {
        case EMANE::Any::Type::TYPE_INT64: return EMANE::Any(ffiAny.i64_value);
        case EMANE::Any::Type::TYPE_INT32: return EMANE::Any(static_cast<int32_t>(ffiAny.i64_value));
        case EMANE::Any::Type::TYPE_INT16: return EMANE::Any(static_cast<int16_t>(ffiAny.i64_value));
        case EMANE::Any::Type::TYPE_INT8: return EMANE::Any(static_cast<int8_t>(ffiAny.i64_value));
        case EMANE::Any::Type::TYPE_UINT64: return EMANE::Any(ffiAny.u64_value);
        case EMANE::Any::Type::TYPE_UINT32: return EMANE::Any(static_cast<uint32_t>(ffiAny.u64_value));
        case EMANE::Any::Type::TYPE_UINT16: return EMANE::Any(static_cast<uint16_t>(ffiAny.u64_value));
        case EMANE::Any::Type::TYPE_UINT8: return EMANE::Any(static_cast<uint8_t>(ffiAny.u64_value));
        case EMANE::Any::Type::TYPE_FLOAT: return EMANE::Any(static_cast<float>(ffiAny.d_value));
        case EMANE::Any::Type::TYPE_DOUBLE: return EMANE::Any(ffiAny.d_value);
        case EMANE::Any::Type::TYPE_BOOL: return EMANE::Any(ffiAny.u64_value != 0);
        case EMANE::Any::Type::TYPE_STRING:
        case EMANE::Any::Type::TYPE_INET_ADDR:
            if (ffiAny.s_value) {
                return EMANE::Any::create(std::string(ffiAny.s_value), type);
            } else {
                return EMANE::Any::create(std::string(""), type);
            }
        default: return EMANE::Any(ffiAny.i64_value);
    }
}

static std::vector<EMANE::Any> convertFfiAnyArrayToVector(const FfiAnyArray& array) {
    std::vector<EMANE::Any> result;
    for(size_t i = 0; i < array.len; ++i) {
        result.push_back(convertFfiToAny(array.data[i]));
    }
    return result;
}

static EMANE::ConfigurationUpdate convertFfiUpdateToUpdate(const FfiConfigUpdate& update) {
    EMANE::ConfigurationUpdate result;
    for(size_t i = 0; i < update.len; ++i) {
        const auto& item = update.data[i];
        result.push_back(std::make_pair(std::string(item.name), convertFfiAnyArrayToVector(item.values)));
    }
    return result;
}

extern "C" {
    bool emane_c_config_call_validator(void* pValidator, const FfiConfigUpdate* update, char* error_buf, size_t error_buf_len) {
        if (!pValidator || !update) return false;
        
        auto* validator = static_cast<EMANE::ConfigurationValidator*>(pValidator);
        EMANE::ConfigurationUpdate cppUpdate = convertFfiUpdateToUpdate(*update);
        
        try {
            auto ret = (*validator)(cppUpdate);
            if (!ret.second) {
                if (error_buf && error_buf_len > 0) {
                    strncpy(error_buf, ret.first.c_str(), error_buf_len - 1);
                    error_buf[error_buf_len - 1] = '\0';
                }
                return false;
            }
            return true;
        } catch(const std::exception& e) {
            if (error_buf && error_buf_len > 0) {
                strncpy(error_buf, e.what(), error_buf_len - 1);
                error_buf[error_buf_len - 1] = '\0';
            }
            return false;
        }
    }

    void emane_c_config_process_configuration(void* pRunningStateMutable, const FfiConfigUpdate* update) {
        if (!pRunningStateMutable || !update) return;
        auto* rsm = static_cast<EMANE::RunningStateMutable*>(pRunningStateMutable);
        EMANE::ConfigurationUpdate cppUpdate = convertFfiUpdateToUpdate(*update);
        rsm->processConfiguration(cppUpdate);
    }
}

// C++ Implementation Class
EMANE::ConfigurationService::ConfigurationService(){}

void EMANE::ConfigurationService::registerRunningStateMutable(BuildId buildId,
                                                              RunningStateMutable * pRunningStateMutable)
{
  std::lock_guard<std::mutex> m(mutex_);
  emane_rs_config_register_running_state_mutable(buildId, pRunningStateMutable);
}

void EMANE::ConfigurationService::registerNumericAny(BuildId buildId,
                                                     const std::string & sName,
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
  std::lock_guard<std::mutex> m(mutex_);
  
  std::vector<FfiAny> ffiValues(values.size());
  std::vector<std::string> stringStorage;
  
  for(size_t i = 0; i < values.size(); ++i) {
      convertAnyToFfi(values[i], ffiValues[i], stringStorage);
  }
  
  FfiAny ffiMin, ffiMax;
  convertAnyToFfi(minValue, ffiMin, stringStorage);
  convertAnyToFfi(maxValue, ffiMax, stringStorage);
  
  FfiAnyArray ffiArray{ffiValues.empty() ? nullptr : ffiValues.data(), ffiValues.size()};
  
  char error_buf[256] = {0};
  emane_rs_config_register_numeric_any(buildId, sName.c_str(), static_cast<int32_t>(type), static_cast<uint64_t>(properties), ffiArray, sUsage.c_str(), ffiMin, ffiMax, minOccurs, maxOccurs, sRegexPattern.c_str(), error_buf, sizeof(error_buf));
  
  if (error_buf[0] != '\0') {
      throw makeException<RegistrarException>("%s", error_buf);
  }
}

void EMANE::ConfigurationService::registerNonNumericAny(BuildId buildId,
                                                        const std::string & sName,
                                                        Any::Type type,
                                                        const ConfigurationProperties & properties,
                                                        const std::vector<Any> & values,
                                                        const std::string & sUsage,
                                                        std::size_t minOccurs,
                                                        std::size_t maxOccurs,
                                                        const std::string & sRegexPattern)
{
  std::lock_guard<std::mutex> m(mutex_);
  
  std::vector<FfiAny> ffiValues(values.size());
  std::vector<std::string> stringStorage;
  
  for(size_t i = 0; i < values.size(); ++i) {
      convertAnyToFfi(values[i], ffiValues[i], stringStorage);
  }
  
  FfiAnyArray ffiArray{ffiValues.empty() ? nullptr : ffiValues.data(), ffiValues.size()};
  
  char error_buf[256] = {0};
  emane_rs_config_register_non_numeric_any(buildId, sName.c_str(), static_cast<int32_t>(type), static_cast<uint64_t>(properties), ffiArray, sUsage.c_str(), minOccurs, maxOccurs, sRegexPattern.c_str(), error_buf, sizeof(error_buf));
  
  if (error_buf[0] != '\0') {
      throw makeException<RegistrarException>("%s", error_buf);
  }
}

void EMANE::ConfigurationService::registerAny(BuildId buildId,
                                              const std::string & sName,
                                              ConfigurationInfo && configurationInfo)
{
   // Internal method - unused directly
}

EMANE::ConfigurationManifest
EMANE::ConfigurationService::getConfigurationManifest(BuildId buildId) const
{
  std::lock_guard<std::mutex> m(mutex_);
  
  FfiConfigManifest manifest = emane_rs_config_get_manifest(buildId);
  EMANE::ConfigurationManifest result;
  
  for(size_t i = 0; i < manifest.len; ++i) {
      const auto& info = manifest.data[i];
      if (info.has_min_max) {
          result.push_back(ConfigurationInfo(
              std::string(info.name),
              static_cast<EMANE::Any::Type>(info.any_type),
              static_cast<EMANE::ConfigurationProperties>(info.properties),
              convertFfiAnyArrayToVector(info.values),
              std::string(info.usage),
              convertFfiToAny(info.min_value),
              convertFfiToAny(info.max_value),
              info.min_occurs,
              info.max_occurs,
              std::string(info.regex_pattern ? info.regex_pattern : "")
          ));
      } else {
          result.push_back(ConfigurationInfo(
              std::string(info.name),
              static_cast<EMANE::Any::Type>(info.any_type),
              static_cast<EMANE::ConfigurationProperties>(info.properties),
              convertFfiAnyArrayToVector(info.values),
              std::string(info.usage),
              info.min_occurs,
              info.max_occurs,
              std::string(info.regex_pattern ? info.regex_pattern : "")
          ));
      }
  }
  
  emane_rs_config_free_manifest(manifest);
  return result;
}

std::vector<std::pair<std::string,std::vector<EMANE::Any>>>
  EMANE::ConfigurationService::queryConfiguration(BuildId buildId,
                                                  const std::vector<std::string> & names) const
{
  std::lock_guard<std::mutex> m(mutex_);
  
  std::vector<const char*> c_names;
  for(const auto& name : names) {
      c_names.push_back(name.c_str());
  }
  
  FfiStringArray ffiNames{c_names.empty() ? nullptr : c_names.data(), c_names.size()};
  FfiConfigUpdate update = emane_rs_config_query(buildId, ffiNames);
  
  auto result = convertFfiUpdateToUpdate(update);
  emane_rs_config_free_update(update);
  return result;
}

EMANE::ConfigurationUpdate
EMANE::ConfigurationService::buildUpdates(BuildId buildId,
                                          const ConfigurationUpdateRequest & parameters)
{
  std::lock_guard<std::mutex> m(mutex_);
  
  std::vector<FfiConfigUpdateReqItem> ffiItems(parameters.size());
  std::vector<std::vector<const char*>> stringArrays(parameters.size());
  
  for(size_t i = 0; i < parameters.size(); ++i) {
      ffiItems[i].name = parameters[i].first.c_str();
      for(const auto& val : parameters[i].second) {
          stringArrays[i].push_back(val.c_str());
      }
      ffiItems[i].values.data = stringArrays[i].empty() ? nullptr : stringArrays[i].data();
      ffiItems[i].values.len = stringArrays[i].size();
  }
  
  FfiConfigUpdateReq req{ffiItems.empty() ? nullptr : ffiItems.data(), ffiItems.size()};
  
  char error_buf[256] = {0};
  FfiConfigUpdate update = emane_rs_config_build_updates(buildId, req, error_buf, sizeof(error_buf));
  
  if (error_buf[0] != '\0') {
      throw makeException<ConfigurationException>("%s", error_buf);
  }
  
  auto result = convertFfiUpdateToUpdate(update);
  emane_rs_config_free_update(update);
  return result;
}

void EMANE::ConfigurationService::update(BuildId buildId,
                                         const ConfigurationUpdate & updates)
{
  std::lock_guard<std::mutex> m(mutex_);
  
  std::vector<FfiConfigItemUpdate> ffiItems(updates.size());
  std::vector<std::vector<FfiAny>> anyArrays(updates.size());
  std::vector<std::string> stringStorage;
  
  for(size_t i = 0; i < updates.size(); ++i) {
      ffiItems[i].name = updates[i].first.c_str();
      anyArrays[i].resize(updates[i].second.size());
      for(size_t j = 0; j < updates[i].second.size(); ++j) {
          convertAnyToFfi(updates[i].second[j], anyArrays[i][j], stringStorage);
      }
      ffiItems[i].values.data = anyArrays[i].empty() ? nullptr : anyArrays[i].data();
      ffiItems[i].values.len = anyArrays[i].size();
  }
  
  FfiConfigUpdate req{ffiItems.empty() ? nullptr : ffiItems.data(), ffiItems.size()};
  
  char error_buf[256] = {0};
  bool success = emane_rs_config_update(buildId, req, error_buf, sizeof(error_buf));
  
  if (!success && error_buf[0] != '\0') {
      throw makeException<ConfigurationException>("%s", error_buf);
  }
}

void EMANE::ConfigurationService::registerValidator(BuildId buildId, ConfigurationValidator validator)
{
  std::lock_guard<std::mutex> m(mutex_);
  auto* heapValidator = new ConfigurationValidator(validator);
  emane_rs_config_register_validator(buildId, heapValidator);
}
