extern "C" void emane_rs_tdma_packet_publisher_clear(void* impl, int table_type, int queue_index);
#include "packetstatuspublisherimpl.h"

extern "C" void* emane_rs_tdma_packet_publisher_create();
extern "C" void emane_rs_tdma_packet_publisher_destroy(void*);
extern "C" void emane_rs_tdma_packet_publisher_inbound(void* impl, EMANE::NEMId src, EMANE::NEMId dst, EMANE::Priority priority, size_t size, EMANE::Models::TDMA::PacketStatusPublisher::InboundAction action);
extern "C" void emane_rs_tdma_packet_publisher_outbound(void* impl, EMANE::NEMId src, EMANE::NEMId dst, EMANE::Priority priority, size_t size, EMANE::Models::TDMA::PacketStatusPublisher::OutboundAction action);
extern "C" void emane_rs_tdma_packet_publisher_register(void* impl, EMANE::StatisticRegistrar * pRegistrar, 
    EMANE::StatisticTable<EMANE::NEMId>** broadcastAcceptTables,
    EMANE::StatisticTable<EMANE::NEMId>** broadcastDropTables,
    EMANE::StatisticTable<EMANE::NEMId>** unicastAcceptTables,
    EMANE::StatisticTable<EMANE::NEMId>** unicastDropTables);

extern "C" void emane_tdma_packet_table_add_row(void* pTable, EMANE::NEMId key, uint64_t num_cols, uint64_t* values) {
  auto table = static_cast<EMANE::StatisticTable<EMANE::NEMId>*>(pTable);
  std::vector<EMANE::Any> any_values;
  any_values.reserve(num_cols);
  any_values.push_back(EMANE::Any{key});
  for(size_t i = 1; i < num_cols; ++i) {
      any_values.push_back(EMANE::Any{static_cast<long>(values[i])});
  }
  table->addRow(key, any_values);
}

extern "C" void emane_tdma_packet_table_set_cell(void* pTable, EMANE::NEMId key, size_t column, uint64_t value) {
  auto table = static_cast<EMANE::StatisticTable<EMANE::NEMId>*>(pTable);
  table->setCell(key, column, EMANE::Any{static_cast<long>(value)});
}

namespace
{
  const EMANE::StatisticTableLabels PacketAcceptLabels =
    {
      "NEM",
      "Num Bytes Tx",
      "Num Bytes Rx"
    };

  const EMANE::StatisticTableLabels PacketDropLabels =
    {
      "NEM",
      "SINR",
      "Reg Id",
      "Dst MAC",
      "Queue Overflow",
      "Bad Control",
      "Bad Spectrum Query",
      "Flow Control",
      "Big",
      "Long",
      "Freq",
      "Slot Error",
      "Miss Fragment"
    };
}

EMANE::Models::TDMA::PacketStatusPublisherImpl::PacketStatusPublisherImpl():
  pImpl_{emane_rs_tdma_packet_publisher_create()}
{}

EMANE::Models::TDMA::PacketStatusPublisherImpl::~PacketStatusPublisherImpl()
{
  emane_rs_tdma_packet_publisher_destroy(pImpl_);
}

void EMANE::Models::TDMA::PacketStatusPublisherImpl::registerStatistics(StatisticRegistrar & statisticRegistrar)
{
  for(int queueIndex = 0; queueIndex < QUEUE_COUNT; ++queueIndex)
    {
      broadcastAcceptTables_[queueIndex] =
        statisticRegistrar.registerTable<NEMId>("BroadcastByteAcceptTable" + std::to_string(queueIndex),
                                                PacketAcceptLabels,
                                                [this, queueIndex](StatisticTablePublisher * pTable) { emane_rs_tdma_packet_publisher_clear(pImpl_, 0, queueIndex); pTable->clear(); },
                                                "Broadcast bytes accepted");

      unicastAcceptTables_[queueIndex] =
        statisticRegistrar.registerTable<NEMId>("UnicastByteAcceptTable" + std::to_string(queueIndex),
                                                PacketAcceptLabels,
                                                [this, queueIndex](StatisticTablePublisher * pTable) { emane_rs_tdma_packet_publisher_clear(pImpl_, 2, queueIndex); pTable->clear(); },
                                                "Unicast bytes accepted");

      broadcastDropTables_[queueIndex] =
        statisticRegistrar.registerTable<NEMId>("BroadcastByteDropTable" + std::to_string(queueIndex),
                                                PacketDropLabels,
                                                [this, queueIndex](StatisticTablePublisher * pTable) { emane_rs_tdma_packet_publisher_clear(pImpl_, 1, queueIndex); pTable->clear(); },
                                                "Broadcast bytes dropped");

      unicastDropTables_[queueIndex] =
        statisticRegistrar.registerTable<NEMId>("UnicastByteDropTable" + std::to_string(queueIndex),
                                                PacketDropLabels,
                                                [this, queueIndex](StatisticTablePublisher * pTable) { emane_rs_tdma_packet_publisher_clear(pImpl_, 3, queueIndex); pTable->clear(); },
                                                "Unicast bytes dropped");
    }
    emane_rs_tdma_packet_publisher_register(pImpl_, &statisticRegistrar, broadcastAcceptTables_.data(), broadcastDropTables_.data(), unicastAcceptTables_.data(), unicastDropTables_.data());
}

void EMANE::Models::TDMA::PacketStatusPublisherImpl::inbound(NEMId src, const MessageComponent & component, InboundAction action)
{
  inbound(src, component.getDestination(), component.getPriority(), component.getData().size(), action);
}

void EMANE::Models::TDMA::PacketStatusPublisherImpl::inbound(NEMId src, const MessageComponents & components, InboundAction action)
{
  for(const auto & component : components) {
    inbound(src, component.getDestination(), component.getPriority(), component.getData().size(), action);
  }
}

void EMANE::Models::TDMA::PacketStatusPublisherImpl::inbound(NEMId src, NEMId dst, Priority priority, size_t size, InboundAction action)
{
  emane_rs_tdma_packet_publisher_inbound(pImpl_, src, dst, priority, size, action);
}

void EMANE::Models::TDMA::PacketStatusPublisherImpl::outbound(NEMId src, NEMId dst, Priority priority, size_t size, OutboundAction action)
{
  emane_rs_tdma_packet_publisher_outbound(pImpl_, src, dst, priority, size, action);
}

void EMANE::Models::TDMA::PacketStatusPublisherImpl::outbound(NEMId src, const MessageComponents & components, OutboundAction action)
{
  for(const auto & component : components) {
    outbound(src, component.getDestination(), component.getPriority(), component.getData().size(), action);
  }
}
