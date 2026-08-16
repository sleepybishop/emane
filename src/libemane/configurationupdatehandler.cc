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

#include "configurationupdatehandler.h"
#include "rust_ffi.h"
#include "emane/serializationexception.h"
#include "anyutils.h"

std::string
EMANE::ControlPort::ConfigurationUpdateHandler::
process(const EMANERemoteControlPortAPI::Request::Update::Configuration & configuration,
        std::uint32_t u32Sequence,
        std::uint32_t u32Reference)
{
  bool bUpdate{true};

  std::vector<std::pair<std::string,std::vector<Any>>> updates;

  EMANERemoteControlPortAPI::Response response;

  for(const auto & parameter : configuration.parameters())
    {
      std::vector<Any> anys;

      for(const auto & any : parameter.values())
        {
          try
            {
              anys.push_back(toAny(any));
            }
          catch(AnyException & exp)
            {
              response.set_type(EMANERemoteControlPortAPI::Response::TYPE_RESPONSE_ERROR);

              auto pError = response.mutable_error();

              pError->set_type(EMANERemoteControlPortAPI::Response::Error::TYPE_ERROR_PARAMETER);

              pError->set_description(exp.what());

              bUpdate = false;

              break;
            }
        }

      updates.push_back(std::make_pair(parameter.name(),std::move(anys)));
    }

  if(bUpdate)
    {
      std::vector<FfiConfigItemUpdate> ffiItems;
      std::vector<std::vector<FfiAny>> anyArrays(updates.size());
      std::vector<std::string> stringStorage;
      
      for(size_t i = 0; i < updates.size(); ++i) {
          FfiConfigItemUpdate ffiItem;
          ffiItem.name = updates[i].first.c_str();
          anyArrays[i].resize(updates[i].second.size());
          for(size_t j = 0; j < updates[i].second.size(); ++j) {
              const auto& any = updates[i].second[j];
              FfiAny& ffiAny = anyArrays[i][j];
              
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
          ffiItem.values.data = anyArrays[i].empty() ? nullptr : anyArrays[i].data();
          ffiItem.values.len = anyArrays[i].size();
          ffiItems.push_back(ffiItem);
      }
      
      FfiConfigUpdate ffiUpdate{ffiItems.empty() ? nullptr : ffiItems.data(), ffiItems.size()};
      char error_buf[256] = {0};
      
      bool success = emane_rs_config_update(configuration.buildid(), ffiUpdate, error_buf, sizeof(error_buf));

      if (success)
        {
          response.set_type(EMANERemoteControlPortAPI::Response::TYPE_RESPONSE_UPDATE);
        }
      else
        {
          response.set_type(EMANERemoteControlPortAPI::Response::TYPE_RESPONSE_ERROR);

          auto pError = response.mutable_error();

          pError->set_type(EMANERemoteControlPortAPI::Response::Error::TYPE_ERROR_PARAMETER);

          pError->set_description(error_buf);
        }
    }

  response.set_reference(u32Reference);

  response.set_sequence(u32Sequence);

  std::string sSerialization;

  if(!response.SerializeToString(&sSerialization))
    {
      throw SerializationException("unable to serialize configuration update response");
    }

  return sSerialization;
}
