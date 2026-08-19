#include "queuestatuspublisher.h"

extern "C" void* emane_rs_tdma_queue_publisher_create();
extern "C" void emane_rs_tdma_queue_publisher_destroy(void*);
extern "C" void emane_rs_tdma_queue_publisher_register(void* impl, EMANE::StatisticRegistrar * pRegistrar, 
    void* pQueueStatusTable, void* pQueueFragmentHistogram,
    void* hw0, void* hw1, void* hw2, void* hw3, void* hw4);

extern "C" void emane_rs_tdma_queue_publisher_drop(void* impl, uint8_t u8Queue, int reason, size_t count);
extern "C" void emane_rs_tdma_queue_publisher_enqueue(void* impl, uint8_t u8Queue);

extern "C" void emane_tdma_queue_table_add_row_status(void* pTable, uint8_t key, uint64_t v1, uint64_t v2, uint64_t v3, uint64_t v4, uint64_t v5, uint64_t v6, uint64_t v7, uint64_t v8, uint64_t v9) {
  auto table = static_cast<EMANE::StatisticTable<std::uint8_t>*>(pTable);
  table->addRow(key, {EMANE::Any{key}, EMANE::Any{static_cast<long>(v1)}, EMANE::Any{static_cast<long>(v2)}, EMANE::Any{static_cast<long>(v3)}, EMANE::Any{static_cast<long>(v4)}, EMANE::Any{static_cast<long>(v5)}, EMANE::Any{static_cast<long>(v6)}, EMANE::Any{static_cast<long>(v7)}, EMANE::Any{static_cast<long>(v8)}, EMANE::Any{static_cast<long>(v9)}});
}

extern "C" void emane_tdma_queue_table_set_cell(void* pTable, uint8_t key, size_t column, uint64_t value) {
  auto table = static_cast<EMANE::StatisticTable<std::uint8_t>*>(pTable);
  table->setCell(key, column, EMANE::Any{static_cast<long>(value)});
}

extern "C" uint64_t emane_tdma_numeric_u64_get(void* pNumeric) {
  return static_cast<EMANE::StatisticNumeric<std::uint64_t>*>(pNumeric)->get();
}
extern "C" void emane_tdma_numeric_u64_set(void* pNumeric, uint64_t val) {
  *static_cast<EMANE::StatisticNumeric<std::uint64_t>*>(pNumeric) = val;
}

EMANE::Models::TDMA::QueueStatusPublisher::QueueStatusPublisher():
  pImpl_{emane_rs_tdma_queue_publisher_create()}
{}

EMANE::Models::TDMA::QueueStatusPublisher::~QueueStatusPublisher()
{
  emane_rs_tdma_queue_publisher_destroy(pImpl_);
}

void EMANE::Models::TDMA::QueueStatusPublisher::registerStatistics(StatisticRegistrar & statisticRegistrar)
{
  auto pQueueStatusTable =
    statisticRegistrar.registerTable<std::uint8_t>("QueueStatusTable",
      {"Queue","Enqueued","Dequeued","Overflow","Too Big","0","1","2","3","4"},
      StatisticProperties::NONE,
      "Shows for each queue the number of packets enqueued, dequeued,"
      " dropped due to queue overflow (enqueue), dropped due to too big"
      " (dequeue) and which slot classes fragments are being transmitted.");

  auto pQueueFragmentHistogram =
    statisticRegistrar.registerTable<std::uint8_t>("QueueFragmentHistogram",
      {"Queue","1","2","3","4","5","6","7","8","9",">9"},
      StatisticProperties::NONE,
      "Shows a per queue histogram of the number of message components required to transmit packets.");

  auto hw0 = statisticRegistrar.registerNumeric<std::uint64_t>("highWaterMarkQueue0", StatisticProperties::CLEARABLE, "High water mark queue 0");
  auto hw1 = statisticRegistrar.registerNumeric<std::uint64_t>("highWaterMarkQueue1", StatisticProperties::CLEARABLE, "High water mark queue 1");
  auto hw2 = statisticRegistrar.registerNumeric<std::uint64_t>("highWaterMarkQueue2", StatisticProperties::CLEARABLE, "High water mark queue 2");
  auto hw3 = statisticRegistrar.registerNumeric<std::uint64_t>("highWaterMarkQueue3", StatisticProperties::CLEARABLE, "High water mark queue 3");
  auto hw4 = statisticRegistrar.registerNumeric<std::uint64_t>("highWaterMarkQueue4", StatisticProperties::CLEARABLE, "High water mark queue 4");

  emane_rs_tdma_queue_publisher_register(pImpl_, &statisticRegistrar, pQueueStatusTable, pQueueFragmentHistogram, hw0, hw1, hw2, hw3, hw4);
}

void EMANE::Models::TDMA::QueueStatusPublisher::drop(std::uint8_t u8Queue, DropReason reason, size_t count)
{
  emane_rs_tdma_queue_publisher_drop(pImpl_, u8Queue, reason == DropReason::DROP_OVERFLOW ? 0 : 1, count);
}

void EMANE::Models::TDMA::QueueStatusPublisher::enqueue(std::uint8_t u8Queue)
{
  emane_rs_tdma_queue_publisher_enqueue(pImpl_, u8Queue);
}

extern "C" void emane_rs_tdma_queue_publisher_dequeue(void* impl, uint8_t u8RequestQueue, uint8_t u8ActualQueue, size_t num_components, const uint8_t* more_fragments, const size_t* fragment_indices);

void EMANE::Models::TDMA::QueueStatusPublisher::dequeue(std::uint8_t u8RequestQueue, std::uint8_t u8ActualQueue, const MessageComponents & components)
{
  std::vector<uint8_t> more_fragments;
  std::vector<size_t> fragment_indices;
  more_fragments.reserve(components.size());
  fragment_indices.reserve(components.size());
  for(const auto & component : components) {
    more_fragments.push_back(component.isMoreFragments() ? 1 : 0);
    fragment_indices.push_back(component.getFragmentIndex());
  }
  emane_rs_tdma_queue_publisher_dequeue(pImpl_, u8RequestQueue, u8ActualQueue, components.size(), more_fragments.data(), fragment_indices.data());
}
extern "C" void emane_tdma_queue_table_add_row_fragment(void* pTable, uint8_t key, uint64_t v1, uint64_t v2, uint64_t v3, uint64_t v4, uint64_t v5, uint64_t v6, uint64_t v7, uint64_t v8, uint64_t v9, uint64_t v10) {
  auto table = static_cast<EMANE::StatisticTable<std::uint8_t>*>(pTable);
  table->addRow(key, {EMANE::Any{key}, EMANE::Any{static_cast<long>(v1)}, EMANE::Any{static_cast<long>(v2)}, EMANE::Any{static_cast<long>(v3)}, EMANE::Any{static_cast<long>(v4)}, EMANE::Any{static_cast<long>(v5)}, EMANE::Any{static_cast<long>(v6)}, EMANE::Any{static_cast<long>(v7)}, EMANE::Any{static_cast<long>(v8)}, EMANE::Any{static_cast<long>(v9)}, EMANE::Any{static_cast<long>(v10)}});
}
