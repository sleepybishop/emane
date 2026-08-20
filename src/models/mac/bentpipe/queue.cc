/*
 * Copyright (c) 2015,2023 - Adjacent Link LLC, Bridgewater, New Jersey
 * All rights reserved.
 */

#include "queue.h"

EMANE::Models::BentPipe::Queue::Queue():
  u16QueueDepth_{},
  bFragment_{},
  u64Counter_{},
  currentBytes_{}{}

void EMANE::Models::BentPipe::Queue::initialize(std::uint16_t, bool, bool) {}

std::pair<std::unique_ptr<EMANE::DownstreamPacket>,bool>
EMANE::Models::BentPipe::Queue::enqueue(DownstreamPacket && pkt)
{
  return {std::unique_ptr<EMANE::DownstreamPacket>{new DownstreamPacket{std::move(pkt)}}, true};
}

std::tuple<EMANE::Models::BentPipe::MessageComponents,
           size_t,
           std::list<std::unique_ptr<EMANE::DownstreamPacket>>>
EMANE::Models::BentPipe::Queue::dequeue(size_t, bool)
{
  return std::make_tuple(MessageComponents{}, 0, std::list<std::unique_ptr<DownstreamPacket>>{});
}

std::pair<EMANE::Models::BentPipe::MessageComponent,size_t>
EMANE::Models::BentPipe::Queue::fragmentPacket(DownstreamPacket *,
                                               MetaInfo *,
                                               std::uint64_t,
                                               size_t)
{
  return {MessageComponent{0, {}}, 0};
}

std::tuple<size_t,size_t> EMANE::Models::BentPipe::Queue::getStatus() const
{
  return std::make_tuple(0,0);
}
