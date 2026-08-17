#include "logservice.h"
#include <cstdio>
#include <cstdarg>
#include <vector>

extern "C" {
    void emane_rs_log(int level, const char* msg);
    void emane_rs_log_set_level(int level);
    void emane_rs_log_redirect(const char* file);
    void emane_rs_log_open();
    bool emane_rs_log_is_allowed(int level);
}

EMANE::LogService::LogService() {}
EMANE::LogService::~LogService() {}

void EMANE::LogService::log(LogLevel level, const char *fmt, ...) {
    if (!isLogAllowed(level)) return;
    va_list ap;
    va_start(ap, fmt);
    vlog(level, fmt, ap);
    va_end(ap);
}

void EMANE::LogService::vlog(LogLevel level, const char *fmt, va_list ap) {
    if (!isLogAllowed(level)) return;
    char buf[4096];
    vsnprintf(buf, sizeof(buf), fmt, ap);
    emane_rs_log(static_cast<int>(level), buf);
}

void EMANE::LogService::log(LogLevel level, const Strings & strings) {
    if (!isLogAllowed(level)) return;
    std::string sMessage;
    for (const auto& s : strings) {
        sMessage += s + " ";
    }
    emane_rs_log(static_cast<int>(level), sMessage.c_str());
}

void EMANE::LogService::setLogLevel(LogLevel level) {
    emane_rs_log_set_level(static_cast<int>(level));
}

void EMANE::LogService::redirectLogsToFile(const std::string & file) {
    emane_rs_log_redirect(file.c_str());
}

void EMANE::LogService::open() {
    emane_rs_log_open();
}

bool EMANE::LogService::isLogAllowed(LogLevel level) const {
    return emane_rs_log_is_allowed(static_cast<int>(level));
}

void EMANE::LogService::log_i(LogLevel level, const Strings & strings) {
    log(level, strings);
}
