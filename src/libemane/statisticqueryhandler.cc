/*
 * Copyright (c) 2013,2016 - Adjacent Link LLC, Bridgewater, New Jersey
 * All rights reserved.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions
 * are met:
 *
 * * Redistributions of source code must retain the above copyright
 *   notice, this list of conditions and the following disclaimer.
 * * Redistributions in binary form must reproduce the above copyright
 *   notice, this list of conditions and the following disclaimer in
 *   the documentation and/or other materials provided with the
 *   distribution.
 * * Neither the name of Adjacent Link LLC nor the names of its
 *   contributors may be used to endorse or promote products derived
 *   from this software without specific prior written permission.
 *
 * THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS
 * "AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT
 * LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS
 * FOR A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE
 * COPYRIGHT OWNER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT,
 * INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING,
 * BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES;
 * LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER
 * CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT
 * LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN
 * ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE
 * POSSIBILITY OF SUCH DAMAGE.
 */

#include "statisticqueryhandler.h"
#include "rust_ffi.h"
#include "emane/serializationexception.h"
#include "emane/registrarexception.h"
#include "anyutils.h"

std::string
EMANE::ControlPort::StatisticQueryHandler::process(const EMANERemoteControlPortAPI::Request::Query::Statistic & statistic,
                                                   std::uint32_t u32Sequence,
                                                   std::uint32_t u32Reference)
{
  std::vector<const char*> c_names;
  for(int i = 0; i < statistic.names_size(); ++i)
    {
      c_names.push_back(statistic.names(i).c_str());
    }

  EMANERemoteControlPortAPI::Response response;

  try
    {
      FfiStringArray ffiNames{c_names.empty() ? nullptr : c_names.data(), c_names.size()};
      char err_buf[256] = {0};
      FfiStatisticQueryResult update = emane_rs_statistic_query(statistic.buildid(), ffiNames, err_buf, sizeof(err_buf));

      if (err_buf[0] != '\0')
        {
          response.set_type(EMANERemoteControlPortAPI::Response::TYPE_RESPONSE_ERROR);

          auto pError = response.mutable_error();

          pError->set_type(EMANERemoteControlPortAPI::Response::Error::TYPE_ERROR_PARAMETER);

          pError->set_description(err_buf);
        }
      else
        {
          response.set_type(EMANERemoteControlPortAPI::Response::TYPE_RESPONSE_QUERY);

          auto pQuery = response.mutable_query();

          pQuery->set_type(EMANERemoteControlPortAPI::TYPE_QUERY_STATISTIC);

          auto pStatistic = pQuery->mutable_statistic();

          pStatistic->set_buildid(statistic.buildid());

          for(size_t i = 0; i < update.len; ++i)
            {
              const auto & item = update.data[i];
              auto pElement = pStatistic->add_elements();

              pElement->set_name(item.name);

              auto pValue = pElement->mutable_value();
              
              EMANE::Any any = EMANE::convertFfiToAny(item.value);
              convertToAny(pValue, any);
            }
        }
        
      emane_rs_statistic_free_query_result(update);
    }
  catch(RegistrarException & exp)
    {
      response.set_type(EMANERemoteControlPortAPI::Response::TYPE_RESPONSE_ERROR);

      auto pError = response.mutable_error();

      pError->set_type(EMANERemoteControlPortAPI::Response::Error::TYPE_ERROR_PARAMETER);

      pError->set_description(exp.what());
    }

  response.set_reference(u32Reference);

  response.set_sequence(u32Sequence);

  std::string sSerialization;

  if(!response.SerializeToString(&sSerialization))
    {
      throw SerializationException("unable to serialize statistic query response");
    }

  return sSerialization;
}
