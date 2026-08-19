/*
 * Copyright (c) 2013-2014 - Adjacent Link LLC, Bridgewater, New Jersey
 * Copyright (c) 2008 - DRS CenGen, LLC, Columbia, Maryland
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

#include "downstreamqueue.h"
#include "macconfig.h"
#include "macstatistics.h"

extern "C" {
    void ieee80211abg_downstream_queue_stat_add(void* stat, uint32_t val) {
        if(stat) *static_cast<EMANE::StatisticNumeric<std::uint32_t>*>(stat) += val;
    }

    void ieee80211abg_downstream_queue_stat_set(void* stat, uint32_t val) {
        if(stat) *static_cast<EMANE::StatisticNumeric<std::uint32_t>*>(stat) = val;
    }

    uint32_t ieee80211abg_downstream_queue_stat_get(void* stat) {
        if(stat) return static_cast<EMANE::StatisticNumeric<std::uint32_t>*>(stat)->get();
        return 0;
    }

    void ieee80211abg_downstream_queue_free_entry(void* entry) {
        delete static_cast<EMANE::Models::IEEE80211ABG::DownstreamQueueEntry*>(entry);
    }

    void* ieee80211abg_downstream_queue_create(uint16_t id);
    void ieee80211abg_downstream_queue_destroy(void* queue);
    
    void ieee80211abg_downstream_queue_set_stats(
        void* queue,
        void* pNumUnicastPacketsUnsupported,
        void* pNumUnicastBytesUnsupported,
        void* pNumBroadcastPacketsUnsupported,
        void* pNumBroadcastBytesUnsupported
    );

    void ieee80211abg_downstream_queue_set_category_stats(
        void* queue,
        uint8_t category,
        void* pNumUnicastPacketsTooLarge,
        void* pNumUnicastBytesTooLarge,
        void* pNumBroadcastPacketsTooLarge,
        void* pNumBroadcastBytesTooLarge,
        void* pNumHighWaterMark,
        void* pNumHighWaterMax
    );

    void ieee80211abg_downstream_queue_enqueue(
        void* queue,
        void* entry_ptr,
        uint8_t category,
        size_t length,
        uint16_t destination,
        void** dropped_entries,
        size_t* num_dropped,
        size_t max_dropped
    );

    void* ieee80211abg_downstream_queue_dequeue(void* queue);

    void ieee80211abg_downstream_queue_set_max_capacity(void* queue, size_t max_entries);
    void ieee80211abg_downstream_queue_set_max_capacity_for_category(void* queue, size_t max_entries, uint8_t category);
    void ieee80211abg_downstream_queue_set_max_entry_size(void* queue, size_t max_entry_size);
    void ieee80211abg_downstream_queue_set_max_entry_size_for_category(void* queue, size_t max_entry_size, uint8_t category);
    size_t ieee80211abg_downstream_queue_get_max_capacity(const void* queue);
    size_t ieee80211abg_downstream_queue_get_max_capacity_for_category(const void* queue, uint8_t category);
    size_t ieee80211abg_downstream_queue_get_depth(const void* queue);
    size_t ieee80211abg_downstream_queue_get_depth_for_category(const void* queue, uint8_t category);
    size_t ieee80211abg_downstream_queue_get_available_space(const void* queue);
    size_t ieee80211abg_downstream_queue_get_available_space_for_category(const void* queue, uint8_t category);
    size_t ieee80211abg_downstream_queue_get_num_overflow(void* queue, bool clear);
    size_t ieee80211abg_downstream_queue_get_num_overflow_for_category(void* queue, uint8_t category, bool clear);
    void ieee80211abg_downstream_queue_set_categories(void* queue, uint8_t num_categories);
}

EMANE::Models::IEEE80211ABG::DownstreamQueue::DownstreamQueue(EMANE::NEMId id)
{
    rs_state_ = ieee80211abg_downstream_queue_create(id);
}

EMANE::Models::IEEE80211ABG::DownstreamQueue::~DownstreamQueue()
{
    ieee80211abg_downstream_queue_destroy(rs_state_);
}

void EMANE::Models::IEEE80211ABG::DownstreamQueue::registerStatistics(StatisticRegistrar & statisticRegistrar)
{
    auto pNumUnicastPacketsUnsupported =
        statisticRegistrar.registerNumeric<std::uint32_t>("numUnicastPacketsUnsupported", StatisticProperties::CLEARABLE);
    auto pNumUnicastBytesUnsupported =
        statisticRegistrar.registerNumeric<std::uint32_t>("numUnicastBytesUnsupported", StatisticProperties::CLEARABLE);
    auto pNumBroadcastPacketsUnsupported =
        statisticRegistrar.registerNumeric<std::uint32_t>("numBroadcastPacketsUnsupported", StatisticProperties::CLEARABLE);
    auto pNumBroadcastBytesUnsupported =
        statisticRegistrar.registerNumeric<std::uint32_t>("numBroadcastBytesUnsupported", StatisticProperties::CLEARABLE);

    ieee80211abg_downstream_queue_set_stats(
        rs_state_,
        pNumUnicastPacketsUnsupported,
        pNumUnicastBytesUnsupported,
        pNumBroadcastPacketsUnsupported,
        pNumBroadcastBytesUnsupported
    );

    for(std::uint8_t u8Category = 0; u8Category < MAX_ACCESS_CATEGORIES; ++u8Category)
    {
        std::string sCategory{std::to_string(u8Category)};

        auto pNumUnicastPacketsTooLarge =
            statisticRegistrar.registerNumeric<std::uint32_t>("numUnicastPacketsTooLarge" + sCategory, StatisticProperties::CLEARABLE);
        auto pNumUnicastBytesTooLarge =
            statisticRegistrar.registerNumeric<std::uint32_t>("numUnicastBytesTooLarge" + sCategory, StatisticProperties::CLEARABLE);
        auto pNumBroadcastPacketsTooLarge =
            statisticRegistrar.registerNumeric<std::uint32_t>("numBroadcastPacketsTooLarge" + sCategory, StatisticProperties::CLEARABLE);
        auto pNumBroadcastBytesTooLarge =
            statisticRegistrar.registerNumeric<std::uint32_t>("numBroadcastBytesTooLarge" + sCategory, StatisticProperties::CLEARABLE);
        auto pNumHighWaterMark =
            statisticRegistrar.registerNumeric<std::uint32_t>("numHighWaterMark" + sCategory, StatisticProperties::CLEARABLE);
        auto pNumHighWaterMax =
            statisticRegistrar.registerNumeric<std::uint32_t>("numHighWaterMax" + sCategory, StatisticProperties::CLEARABLE);

        ieee80211abg_downstream_queue_set_category_stats(
            rs_state_,
            u8Category,
            pNumUnicastPacketsTooLarge,
            pNumUnicastBytesTooLarge,
            pNumBroadcastPacketsTooLarge,
            pNumBroadcastBytesTooLarge,
            pNumHighWaterMark,
            pNumHighWaterMax
        );
    }
}

void EMANE::Models::IEEE80211ABG::DownstreamQueue::setMaxCapacity(size_t maxEntries, std::uint8_t u8Category)
{
    ieee80211abg_downstream_queue_set_max_capacity_for_category(rs_state_, maxEntries, u8Category);
}

void EMANE::Models::IEEE80211ABG::DownstreamQueue::setMaxCapacity(size_t maxEntries)
{
    ieee80211abg_downstream_queue_set_max_capacity(rs_state_, maxEntries);
}

void EMANE::Models::IEEE80211ABG::DownstreamQueue::setMaxEntrySize(size_t maxEntrySize, std::uint8_t u8Category)
{
    ieee80211abg_downstream_queue_set_max_entry_size_for_category(rs_state_, maxEntrySize, u8Category);
}

void EMANE::Models::IEEE80211ABG::DownstreamQueue::setMaxEntrySize(size_t maxEntrySize)
{
    ieee80211abg_downstream_queue_set_max_entry_size(rs_state_, maxEntrySize);
}

size_t EMANE::Models::IEEE80211ABG::DownstreamQueue::getMaxCapacity(std::uint8_t u8Category)
{
    return ieee80211abg_downstream_queue_get_max_capacity_for_category(rs_state_, u8Category);
}

size_t EMANE::Models::IEEE80211ABG::DownstreamQueue::getMaxCapacity()
{
    return ieee80211abg_downstream_queue_get_max_capacity(rs_state_);
}

size_t EMANE::Models::IEEE80211ABG::DownstreamQueue::getDepth(std::uint8_t u8Category)
{
    return ieee80211abg_downstream_queue_get_depth_for_category(rs_state_, u8Category);
}

size_t EMANE::Models::IEEE80211ABG::DownstreamQueue::getDepth()
{
    return ieee80211abg_downstream_queue_get_depth(rs_state_);
}

size_t EMANE::Models::IEEE80211ABG::DownstreamQueue::getAvailableSpace(std::uint8_t u8Category)
{
    return ieee80211abg_downstream_queue_get_available_space_for_category(rs_state_, u8Category);
}

size_t EMANE::Models::IEEE80211ABG::DownstreamQueue::getAvailableSpace()
{
    return ieee80211abg_downstream_queue_get_available_space(rs_state_);
}

void EMANE::Models::IEEE80211ABG::DownstreamQueue::setCategories(std::uint8_t u8NumCategories)
{
    ieee80211abg_downstream_queue_set_categories(rs_state_, u8NumCategories);
}

std::pair<EMANE::Models::IEEE80211ABG::DownstreamQueueEntry, bool>
EMANE::Models::IEEE80211ABG::DownstreamQueue::dequeue()
{
    void* ptr = ieee80211abg_downstream_queue_dequeue(rs_state_);
    if(ptr) {
        auto entry_ptr = static_cast<DownstreamQueueEntry*>(ptr);
        std::pair<DownstreamQueueEntry, bool> res(std::move(*entry_ptr), true);
        delete entry_ptr;
        return res;
    }
    return {DownstreamQueueEntry(), false};
}

std::vector<EMANE::Models::IEEE80211ABG::DownstreamQueueEntry> 
EMANE::Models::IEEE80211ABG::DownstreamQueue::enqueue(DownstreamQueueEntry & entry)
{
    uint8_t category = entry.u8Category_;
    size_t length = entry.pkt_.length();
    uint16_t destination = entry.pkt_.getPacketInfo().getDestination();

    auto ptr = new DownstreamQueueEntry(std::move(entry));

    void* dropped[256];
    size_t num_dropped = 0;

    ieee80211abg_downstream_queue_enqueue(
        rs_state_,
        ptr,
        category,
        length,
        destination,
        dropped,
        &num_dropped,
        256
    );

    std::vector<DownstreamQueueEntry> result;
    for(size_t i = 0; i < num_dropped; ++i) {
        auto dropped_entry = static_cast<DownstreamQueueEntry*>(dropped[i]);
        result.push_back(std::move(*dropped_entry));
        delete dropped_entry;
    }

    return result;
}

size_t EMANE::Models::IEEE80211ABG::DownstreamQueue::getNumOverFlow(std::uint8_t u8Category, bool bClear)
{
    return ieee80211abg_downstream_queue_get_num_overflow_for_category(rs_state_, u8Category, bClear);
}

size_t EMANE::Models::IEEE80211ABG::DownstreamQueue::getNumOverFlow(bool bClear)
{
    return ieee80211abg_downstream_queue_get_num_overflow(rs_state_, bClear);
}
