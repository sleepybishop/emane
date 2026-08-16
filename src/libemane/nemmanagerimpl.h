/*
 * Copyright (c) 2013-2015,2017,2021 - Adjacent Link LLC, Bridgewater,
 *  New Jersey
 * Copyright (c) 2011 - DRS CenGen, LLC, Columbia, Maryland
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
 * * Neither the name of DRS CenGen, LLC nor the names of its
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

#ifndef EMANEAPPLICATIONNEMMANAGERIMPL_HEADAER_
#define EMANEAPPLICATIONNEMMANAGERIMPL_HEADAER_

#include "emane/application/nemmanager.h"
#include "emane/inetaddr.h"
#include "controlportservice.h"

#include <map>
#include <memory>

namespace EMANE
{
  namespace Application
  {
    /**
     * @class NEMManagerImpl
     *
     * @brief Implementation of Platform interface. Contains and manages NEMs.
     *
     */
    class NEMManagerImpl : public NEMManager
    {
    public:
      NEMManagerImpl(const uuid_t & uuid);

      ~NEMManagerImpl();

      void add(std::unique_ptr<NEM> & pNEM) override;

      void initialize(Registrar & registrar) override;

      void configure(const ConfigurationUpdate & update) override;

      void start() override;

      void postStart() override;

      void stop() override;

      void destroy()
        throw() override;

    private:
      void* pRsNemManager_;
      ControlPort::Service controlPortService_;
      
      // Keep static pointers for C callbacks
      static NEMManagerImpl* pInstance_;
      
    public:
      static NEMManagerImpl* instance() { return pInstance_; }
      ControlPort::Service& getControlPortService() { return controlPortService_; }
    };

    extern "C" {
        void emane_c_nem_start(void* nem_ptr);
        void emane_c_nem_post_start(void* nem_ptr);
        void emane_c_nem_stop(void* nem_ptr);
        void emane_c_nem_destroy(void* nem_ptr);
        void emane_c_control_port_open(const char* port_str);
        void emane_c_control_port_close();
        void emane_c_load_antenna_profile(const char* uri);
        void emane_c_load_spectral_mask(const char* uri);
        void emane_c_ota_manager_open(
            const char* addr,
            const char* device,
            bool loopback,
            std::uint8_t ttl,
            const std::uint8_t* uuid,
            std::uint32_t mtu,
            std::uint16_t part_check_thresh,
            std::uint16_t part_timeout_thresh
        );
        void emane_c_event_service_open(
            const char* addr,
            const char* device,
            std::uint8_t ttl,
            const std::uint8_t* uuid
        );
    }

  }
}

#endif // EMANEAPPLICATIONNEMMANAGERIMPL_HEADAER_
