/*
 * Copyright (c) 2013-2016 - Adjacent Link LLC, Bridgewater, New Jersey
 * Copyright (c) 2008-2012 - DRS CenGen, LLC, Columbia, Maryland
 * All rights reserved.
 */

#ifndef EMANELOGSERVICE_HEADER_
#define EMANELOGSERVICE_HEADER_

#include "emane/logserviceprovider.h"
#include "emane/utils/singleton.h"
#include <string>

namespace EMANE
{
  class LogService : public LogServiceProvider,
                     public Utils::Singleton<LogService>
  {
  public:
    ~LogService();

    void log(LogLevel level,const char *format, ...)
      __attribute__ ((format (printf, 3, 4)));

    void vlog(LogLevel level,const char *format,va_list ap) override;

    void log(LogLevel level, const Strings & strings) override;

    void setLogLevel(LogLevel level);

    void redirectLogsToFile(const std::string & file);

    void open();

  protected:
    LogService();

    bool isLogAllowed(LogLevel level) const override;

    void log_i(LogLevel level, const Strings & strings) override;

  private:
    // No state, all logic in Rust
  };

  using LogServiceSingleton = LogService;
}

#endif //EMANELOGSERVICE_HEADER_
