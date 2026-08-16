/*
 * Copyright (c) 2013 - Adjacent Link LLC, Bridgewater, New Jersey
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

#include "buildidservice.h"
#include "emane/buildexception.h"
#include "rust_ffi.h"

EMANE::BuildIdService::BuildIdService() {}

EMANE::BuildId
EMANE::BuildIdService::assignBuildId(Buildable *pBuildable)
{
  BuildId buildId = emane_rs_buildid_assign();
  pBuildable->setBuildId(buildId);
  return buildId;
}
    
EMANE::BuildId EMANE::BuildIdService::registerBuildable(Application::NEMManager * pNEMManager)
{
  BuildId buildId = assignBuildId(pNEMManager);
  char err_buf[256] = {0};
  emane_rs_buildid_register_nem_manager(buildId, err_buf, sizeof(err_buf));
  if (err_buf[0] != '\0')
    {
      throw BuildException(err_buf);
    }
  return buildId;
}

EMANE::BuildId EMANE::BuildIdService::registerBuildable(NEMLayer * pLayer, ComponentType type,const std::string & sPluginName)
{
  BuildId buildId = assignBuildId(pLayer);
  emane_rs_buildid_register_layer(pLayer->getNEMId(), buildId, static_cast<int32_t>(type), sPluginName.empty() ? nullptr : sPluginName.c_str());
  return buildId;
}

EMANE::BuildId EMANE::BuildIdService::registerBuildable(Application::TransportManager * pTransportManager)
{
  BuildId buildId = assignBuildId(pTransportManager);
  char err_buf[256] = {0};
  emane_rs_buildid_register_transport_manager(buildId, err_buf, sizeof(err_buf));
  if (err_buf[0] != '\0')
    {
      throw BuildException(err_buf);
    }
  return buildId;
}

EMANE::BuildId EMANE::BuildIdService::registerBuildable(Transport * pTransport)
{
  BuildId buildId = assignBuildId(pTransport);
  emane_rs_buildid_register_transport(pTransport->getNEMId(), buildId);
  return buildId;
}

EMANE::BuildId EMANE::BuildIdService::registerBuildable(Application::NEM * pNEM)
{
  BuildId buildId = assignBuildId(pNEM);
  emane_rs_buildid_register_nem(pNEM->getNEMId(), buildId);
  return buildId;
}

EMANE::BuildId EMANE::BuildIdService::registerBuildable(Application::TransportAdapter * pTransportAdapter)
{
  BuildId buildId = assignBuildId(pTransportAdapter);
  emane_rs_buildid_register_transport_adapter(pTransportAdapter->getNEMId(), buildId);
  return buildId;
}

EMANE::BuildId EMANE::BuildIdService::registerBuildable(Application::EventGeneratorManager * pEventGeneratorManager)
{
  BuildId buildId = assignBuildId(pEventGeneratorManager);
  char err_buf[256] = {0};
  emane_rs_buildid_register_event_generator_manager(buildId, err_buf, sizeof(err_buf));
  if (err_buf[0] != '\0')
    {
      throw BuildException(err_buf);
    }
  return buildId;
}

EMANE::BuildId EMANE::BuildIdService::registerBuildable(EventGenerator * pGenerator)
{
  BuildId buildId = assignBuildId(pGenerator);
  emane_rs_buildid_register_event_generator(buildId);
  return buildId;
}

EMANE::BuildId EMANE::BuildIdService::registerBuildable(Application::EventAgentManager * pEventAgentManager)
{
  BuildId buildId = assignBuildId(pEventAgentManager);
  char err_buf[256] = {0};
  emane_rs_buildid_register_event_agent_manager(buildId, err_buf, sizeof(err_buf));
  if (err_buf[0] != '\0')
    {
      throw BuildException(err_buf);
    }
  return buildId;
}

EMANE::BuildId EMANE::BuildIdService::registerBuildable(EventAgent * pEventAgent)
{
  BuildId buildId = assignBuildId(pEventAgent);
  emane_rs_buildid_register_event_agent(buildId);
  return buildId;
}

const EMANE::NEMLayerComponentBuildIdMap & EMANE::BuildIdService::getNEMLayerComponentBuildIdMap() const
{
  // We cannot easily return a reference to a dynamically constructed map from Rust without caching it in C++.
  // But we can implement this by caching it, or by changing the return type in the header.
  // Wait, I will just cache it here in NEMLayerComponentBuildIdMap_ before returning it!
  
  // Actually, since this method is 'const', modifying the cached map requires it to be mutable.
  // I'll cast away const to update the cache.
  auto* self = const_cast<EMANE::BuildIdService*>(this);
  self->NEMLayerComponentBuildIdMap_.clear();

  FfiNEMLayerComponentMap ffiMap = emane_rs_buildid_get_nem_layer_component_map();
  for (size_t i = 0; i < ffiMap.len; ++i) {
      const auto& nem_list = ffiMap.nems[i];
      std::vector<std::tuple<BuildId, ComponentType, std::string>> components;
      for (size_t j = 0; j < nem_list.len; ++j) {
          const auto& comp = nem_list.components[j];
          components.push_back(std::make_tuple(
              comp.build_id, 
              static_cast<ComponentType>(comp.layer_type), 
              comp.plugin_name ? std::string(comp.plugin_name) : std::string()
          ));
      }
      self->NEMLayerComponentBuildIdMap_[nem_list.nem_id] = components;
  }
  emane_rs_buildid_free_nem_layer_component_map(ffiMap);

  return NEMLayerComponentBuildIdMap_;
}

// ... the rest of the commented methods ...
