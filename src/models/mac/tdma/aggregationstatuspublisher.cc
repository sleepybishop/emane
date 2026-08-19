/*
 * Copyright (c) 2015 - Adjacent Link LLC, Bridgewater, New Jersey
 * All rights reserved.
 * ...
 */

#include "aggregationstatuspublisher.h"

extern "C" void* emane_rs_tdma_aggregation_publisher_create();
extern "C" void emane_rs_tdma_aggregation_publisher_destroy(void*);
extern "C" void emane_rs_tdma_aggregation_publisher_update(void* impl, void* table, uint64_t num_components);

extern "C" void emane_tdma_aggregation_table_add_row(void* pTable, uint64_t key, uint64_t components, uint64_t count) {
  auto table = static_cast<EMANE::StatisticTable<std::uint64_t>*>(pTable);
  table->addRow(key, {EMANE::Any{components}, EMANE::Any{static_cast<long>(count)}});
}

extern "C" void emane_tdma_aggregation_table_set_cell(void* pTable, uint64_t key, uint64_t count) {
  auto table = static_cast<EMANE::StatisticTable<std::uint64_t>*>(pTable);
  table->setCell(key, 1, EMANE::Any{static_cast<long>(count)});
}

EMANE::Models::TDMA::AggregationStatusPublisher::AggregationStatusPublisher():
  pAggregationHistogramTable_{},
  pImpl_{emane_rs_tdma_aggregation_publisher_create()}
{}

EMANE::Models::TDMA::AggregationStatusPublisher::~AggregationStatusPublisher()
{
  emane_rs_tdma_aggregation_publisher_destroy(pImpl_);
}

void EMANE::Models::TDMA::AggregationStatusPublisher::registerStatistics(StatisticRegistrar & statisticRegistrar)
{
  pAggregationHistogramTable_ =
    statisticRegistrar.registerTable<std::uint64_t>("PacketComponentAggregationHistogram",
      {"Components","Count"},
      StatisticProperties::NONE,
      "Shows a histogram of the number of components contained in transmitted messages.");
}

void EMANE::Models::TDMA::AggregationStatusPublisher::update(const MessageComponents & components)
{
  emane_rs_tdma_aggregation_publisher_update(pImpl_, pAggregationHistogramTable_, components.size());
}
