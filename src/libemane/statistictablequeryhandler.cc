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

#include "statistictablequeryhandler.h"
#include "rust_ffi.h"
#include "emane/serializationexception.h"
#include "emane/registrarexception.h"
#include "anyutils.h"

std::string
EMANE::ControlPort::StatisticTableQueryHandler::process(const EMANERemoteControlPortAPI::Request::Query::StatisticTable & statisticTable,
                                                        std::uint32_t u32Sequence,
                                                        std::uint32_t u32Reference)
{
  std::vector<const char*> c_names;
  for(int i = 0; i < statisticTable.names_size(); ++i)
    {
      c_names.push_back(statisticTable.names(i).c_str());
    }

  EMANERemoteControlPortAPI::Response response;

  try
    {
      FfiStringArray ffiNames{c_names.empty() ? nullptr : c_names.data(), c_names.size()};
      char err_buf[256] = {0};
      FfiStatisticTableQueryResult update = emane_rs_statistic_query_table(statisticTable.buildid(), ffiNames, err_buf, sizeof(err_buf));

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

          pQuery->set_type(EMANERemoteControlPortAPI::TYPE_QUERY_STATISTICTABLE);

          auto pStatisticTable = pQuery->mutable_statistictable();

          pStatisticTable->set_buildid(statisticTable.buildid());

          for(size_t i = 0; i < update.len; ++i)
            {
              const auto & tableItem = update.data[i];
              auto pTable = pStatisticTable->add_tables();

              pTable->set_name(tableItem.name);

              for(size_t l = 0; l < tableItem.labels.len; ++l)
                {
                  pTable->add_labels(tableItem.labels.data[l]);
                }

              size_t num_cols = tableItem.labels.len;
              size_t num_rows = tableItem.rows_len;

              for(size_t r = 0; r < num_rows; ++r)
                {
                  auto pRow = pTable->add_rows();
                  for(size_t c = 0; c < num_cols; ++c)
                    {
                      const auto & ffiAny = tableItem.rows[r].values.data[c];
                      auto pValue = pRow->add_values();
                      EMANE::Any any = EMANE::convertFfiToAny(ffiAny);
                      convertToAny(pValue, any);
                    }
                }
            }
        }
        
      emane_rs_statistic_free_table_query_result(update);
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
      throw SerializationException("unable to serialize statistic table query response");
    }

  return sSerialization;
}
