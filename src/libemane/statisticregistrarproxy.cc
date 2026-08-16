#include "statisticregistrarproxy.h"
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
    if (err_buf[0] != '\0') {
        throw makeException<RegistrarException>("%s", err_buf);
    }
}

void EMANE::StatisticRegistrarProxy::registerTablePublisher(const std::string & sName,
                                                   const StatisticProperties & properties,
                                                   const std::string & sDescription,
                                                   StatisticTablePublisher * pStatisticTablePublisher,
                                                   std::function<void(StatisticTablePublisher *)> clearFunc)
{
    char err_buf[256] = {0};
    auto pFunc = new std::function<void(StatisticTablePublisher *)>(clearFunc);
    emane_rs_statistic_register_table(buildId_, sName.c_str(), static_cast<uint64_t>(properties), sDescription.c_str(), pStatisticTablePublisher, pFunc, err_buf, sizeof(err_buf));
    if (err_buf[0] != '\0') {
        throw makeException<RegistrarException>("%s", err_buf);
    }
}
