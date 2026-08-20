/*
 * Copyright (c) 2013,2016,2023 - Adjacent Link LLC, Bridgewater,
 *  New Jersey
 * Copyright (c) 2009-2012 - DRS CenGen, LLC, Columbia, Maryland
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

#include "ethernettransport.h"
#include "emane/componenttypes.h"
#include <cstring>

extern "C" {
    void* emane_rs_ethernet_transport_new();
    void emane_rs_ethernet_transport_free(void* state);
    int emane_rs_ethernet_transport_verify_frame(const void* buf, size_t len);
    void emane_rs_ethernet_transport_add_entry(void* state, const std::uint8_t* mac, std::uint16_t nemId);
    bool emane_rs_ethernet_transport_lookup_arp_cache(void* state, const std::uint8_t* mac, std::uint16_t* nemId);
    int emane_rs_ethernet_transport_parse_frame(
        void* state, const void* buf, size_t len,
        bool broadcast_mode, bool arp_cache_mode, std::uint8_t eth_type_arp_priority,
        const void* cpp_obj,
        bool (*query_unknown_cb)(const void*, std::uint16_t, std::uint8_t*),
        std::uint16_t* nem_dest, std::uint8_t* dscp
    );
    void emane_rs_ethernet_transport_update_arp_cache(
        void* state, const void* buf, size_t len, std::uint16_t nem_id,
        bool broadcast_mode, bool arp_cache_mode
    );
}

static bool query_unknown_cb_thunk(const void* obj, std::uint16_t eth_type, std::uint8_t* out_prio) {
    auto eth = static_cast<const EMANE::Transports::Ethernet::EthernetTransport*>(obj);
    return eth->getUnknownPriority(eth_type, *out_prio);
}

EMANE::Transports::Ethernet::EthernetTransport::EthernetTransport(NEMId id,
                                                                  PlatformServiceProvider *pPlatformService):
  Transport(id, pPlatformService),
  bBroadcastMode_(false),
  bArpCacheMode_(true),
  u8EtherTypeARPPriority_{},
  pRustState_(emane_rs_ethernet_transport_new())
{ }

EMANE::Transports::Ethernet::EthernetTransport::~EthernetTransport()
{
  emane_rs_ethernet_transport_free(pRustState_);
}

bool EMANE::Transports::Ethernet::EthernetTransport::getUnknownPriority(std::uint16_t eth_type, std::uint8_t& prio) const
{
   if(auto iter = unknownEtherTypePriorityMap_.find(eth_type);
      iter != unknownEtherTypePriorityMap_.end())
     {
       prio = iter->second;
       return true;
     }
   return false;
}

int EMANE::Transports::Ethernet::EthernetTransport::verifyFrame(const void * buf, size_t len)
{
    int ret = emane_rs_ethernet_transport_verify_frame(buf, len);
    if (ret == -1) {
        // C++ logging equivalent
        if(len < Utils::ETH_HEADER_LEN) {
            LOGGER_STANDARD_LOGGING(pPlatformService_->logService(),
                                    ERROR_LEVEL,
                                    "TRANSPORTI %03d EthernetTransport::%s len %zd < min eth header len %d",
                                    id_,
                                    __func__,
                                    len,
                                    Utils::ETH_HEADER_LEN);
        } else {
           // We could recreate the exact log messages, but since it's just error level, we log a generic or protocol specific error.
           // Since the Rust FFI returns -1, we'll try to emulate the exact original logging if possible, but the prompt says "hollow out ... to act as FFI proxy". 
           // We can just log a generic frame error here, or re-parse in C++ just for the log.
           // Let's just re-parse for the log if len >= ETH_HEADER_LEN to keep the exact same log messages.
           const Utils::EtherHeader *pEthHeader = (Utils::EtherHeader *) buf;
           const std::uint16_t u16ethProtocol = Utils::get_protocol(pEthHeader);
           size_t payload_len = len - Utils::ETH_HEADER_LEN;
           switch(u16ethProtocol) {
               case Utils::ETH_P_IPV4:
                   LOGGER_STANDARD_LOGGING(pPlatformService_->logService(), ERROR_LEVEL,
                                           "TRANSPORTI %03d EthernetTransport::%s ipv4, len %zu < min len %d",
                                           id_, __func__, payload_len, Utils::IPV4_HEADER_LEN);
                   break;
               case Utils::ETH_P_IPV6:
                   LOGGER_STANDARD_LOGGING(pPlatformService_->logService(), ERROR_LEVEL,
                                           "TRANSPORTI %03d EthernetTransport::%s ipv6, len %zu < min len %d",
                                           id_, __func__, payload_len, Utils::IPV6_HEADER_LEN);
                   break;
               case Utils::ETH_P_ARP:
                   LOGGER_STANDARD_LOGGING(pPlatformService_->logService(), ERROR_LEVEL,
                                           "TRANSPORTI %03d EthernetTransport::%s arp, len %zu < len %d",
                                           id_, __func__, payload_len, Utils::ETHARP_HEADER_LEN);
                   break;
           }
        }
    } else if (ret == 1) {
        const Utils::EtherHeader *pEthHeader = (Utils::EtherHeader *) buf;
        const std::uint16_t u16ethProtocol = Utils::get_protocol(pEthHeader);
        LOGGER_VERBOSE_LOGGING(pPlatformService_->logService(),
                               DEBUG_LEVEL, "TRANSPORTI %03d EthernetTransport::%s allow unknown protocol %02X",
                               id_,
                               __func__,
                               u16ethProtocol);
    }
    return ret;
}

int EMANE::Transports::Ethernet::EthernetTransport::parseFrame(const Utils::EtherHeader *pEthHeader,
                                                               NEMId & rNemDestination,
                                                               std::uint8_t & rDspc)
{
    // The original parseFrame method receives pEthHeader which points to the buffer.
    // However, it doesn't receive `len` directly.
    // Wait! parseFrame in EthernetTransport doesn't have `len` argument!
    // Let's look at ethernettransport.h:
    // virtual int parseFrame(const Utils::EtherHeader *pEthHeader, EMANE::NEMId & dst, std::uint8_t & dscp);
    // How did it check length? It didn't! It relied on verifyFrame to have already checked the length.
    // In Rust, emane_rs_ethernet_transport_parse_frame takes `len`.
    // Since we don't have `len` here, we can pass a sufficiently large dummy length (e.g., 65535) 
    // because verifyFrame already verified it, OR we can pass 65535 and rely on the fact that verifyFrame succeeded.
    // Actually, in the original parseFrame:
    // const Utils::Ip4Header *pIpHeader = (Utils::Ip4Header*) ((Utils::EtherHeader*) pEthHeader + 1);
    // rDspc = Utils::get_dscp(pIpHeader);
    // It didn't bounds check because verifyFrame did it!
    // So passing len = 65535 to Rust is safe since it will only read the required headers.
    
    std::uint16_t nem_dest = 0;
    std::uint8_t dscp = 0;
    int ret = emane_rs_ethernet_transport_parse_frame(
        pRustState_, pEthHeader, 65535,
        bBroadcastMode_, bArpCacheMode_, u8EtherTypeARPPriority_,
        this, query_unknown_cb_thunk,
        &nem_dest, &dscp
    );

    rNemDestination = nem_dest;
    rDspc = dscp;

    if (ret == 1) {
        const std::uint16_t u16ethProtocol = Utils::get_protocol(pEthHeader);
        LOGGER_VERBOSE_LOGGING(pPlatformService_->logService(),
                               DEBUG_LEVEL,
                               "TRANSPORTI %03d EthernetTransport::%s allow unknown protocol %02X",
                               id_,
                               __func__,
                               u16ethProtocol);
    }

    return ret;
}

void EMANE::Transports::Ethernet::EthernetTransport::updateArpCache(const Utils::EtherHeader *pEthHeader, NEMId nemId)
{
    emane_rs_ethernet_transport_update_arp_cache(
        pRustState_, pEthHeader, 65535, nemId,
        bBroadcastMode_, bArpCacheMode_
    );
}

void EMANE::Transports::Ethernet::EthernetTransport::addEntry(const Utils::EtherAddr& addr, NEMId nemId)
{
    // The original C++ logged when an entry was added or updated. 
    // The Rust code silently updates it. 
    // To preserve logs precisely, we could read it first or let Rust log via callback.
    // But since it's just DEBUG logging, we can skip it or reimplement it.
    // We'll reimplement it in C++ using lookup to see if it changed.
    std::uint16_t old_nem = 0;
    bool found = emane_rs_ethernet_transport_lookup_arp_cache(pRustState_, reinterpret_cast<const std::uint8_t*>(&addr), &old_nem);
    
    if (!found) {
        LOGGER_VERBOSE_LOGGING(pPlatformService_->logService(),
                               DEBUG_LEVEL,
                               "TRANSPORTI %03d ARPCache::%s added cache entry %s to nem %hu",
                               id_,
                               __func__,
                               Utils::ethaddr_to_string(&addr).c_str(), nemId);
    } else if (old_nem != nemId) {
        LOGGER_VERBOSE_LOGGING(pPlatformService_->logService(),
                               DEBUG_LEVEL,
                               "TRANSPORTI %03d ARPCache::%s updated cache entry %s from nem %hu to nem %hu",
                               id_,
                               __func__,
                               Utils::ethaddr_to_string(&addr).c_str(),
                               old_nem, nemId);
    }
    
    emane_rs_ethernet_transport_add_entry(pRustState_, reinterpret_cast<const std::uint8_t*>(&addr), nemId);
}

EMANE::NEMId EMANE::Transports::Ethernet::EthernetTransport::lookupArpCache(const Utils::EtherAddr *pEtherAddr)
{
    std::uint16_t nem_id = 0;
    bool found = emane_rs_ethernet_transport_lookup_arp_cache(pRustState_, reinterpret_cast<const std::uint8_t*>(pEtherAddr), &nem_id);
    
    if (!found) {
        LOGGER_VERBOSE_LOGGING(pPlatformService_->logService(),
                               DEBUG_LEVEL,
                               "TRANSPORTI %03d EthernetTransport::%s no nem found for %s, using broadcast mac address",
                               id_,
                               __func__,
                               Utils::ethaddr_to_string(pEtherAddr).c_str());
        return NEM_BROADCAST_MAC_ADDRESS;
    } else {
        LOGGER_VERBOSE_LOGGING(pPlatformService_->logService(),
                               DEBUG_LEVEL,
                               "TRANSPORTI %03d EthernetTransport::%s nem %hu found for %s, using %hu",
                               id_,
                               __func__,
                               nem_id,
                               Utils::ethaddr_to_string(pEtherAddr).c_str(),
                               nem_id);
        return nem_id;
    }
}
