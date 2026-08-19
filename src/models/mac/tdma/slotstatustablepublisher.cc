#include "slotstatustablepublisher.h"

extern "C" void* emane_rs_tdma_slot_publisher_create();
extern "C" void emane_rs_tdma_slot_publisher_destroy(void*);
extern "C" void emane_rs_tdma_slot_publisher_register(void* impl, EMANE::StatisticRegistrar * pRegistrar, 
    void* txTable, void* rxTable,
    void* txValid, void* txMissed, void* txTooBig,
    void* rxValid, void* rxMissed, void* rxIdle, void* rxTx, void* rxTooLong, void* rxWrongFreq, void* rxLock);

extern "C" void emane_rs_tdma_slot_publisher_update(void* impl, uint32_t u32RelativeIndex, uint32_t u32RelativeFrameIndex, uint32_t u32RelativeSlotIndex, int status, double dSlotRemainingRatio);
extern "C" void emane_rs_tdma_slot_publisher_clear(void* impl);

extern "C" void emane_tdma_slot_table_add_row_tx(void* pTable, uint32_t key, uint32_t v1, uint32_t v2, uint64_t v3, uint64_t v4, uint64_t v5, uint64_t v6, uint64_t v7, uint64_t v8, uint64_t v9, uint64_t v10, uint64_t v11, uint64_t v12, uint64_t v13) {
  auto table = static_cast<EMANE::StatisticTable<std::uint32_t>*>(pTable);
  table->addRow(key, {EMANE::Any{key}, EMANE::Any{v1}, EMANE::Any{v2}, EMANE::Any{static_cast<long>(v3)}, EMANE::Any{static_cast<long>(v4)}, EMANE::Any{static_cast<long>(v5)}, EMANE::Any{static_cast<long>(v6)}, EMANE::Any{static_cast<long>(v7)}, EMANE::Any{static_cast<long>(v8)}, EMANE::Any{static_cast<long>(v9)}, EMANE::Any{static_cast<long>(v10)}, EMANE::Any{static_cast<long>(v11)}, EMANE::Any{static_cast<long>(v12)}, EMANE::Any{static_cast<long>(v13)}});
}

extern "C" void emane_tdma_slot_table_add_row_rx(void* pTable, uint32_t key, uint32_t v1, uint32_t v2, uint64_t v3, uint64_t v4, uint64_t v5, uint64_t v6, uint64_t v7, uint64_t v8, uint64_t v9, uint64_t v10, uint64_t v11, uint64_t v12, uint64_t v13, uint64_t v14, uint64_t v15, uint64_t v16, uint64_t v17) {
  auto table = static_cast<EMANE::StatisticTable<std::uint32_t>*>(pTable);
  table->addRow(key, {EMANE::Any{key}, EMANE::Any{v1}, EMANE::Any{v2}, EMANE::Any{static_cast<long>(v3)}, EMANE::Any{static_cast<long>(v4)}, EMANE::Any{static_cast<long>(v5)}, EMANE::Any{static_cast<long>(v6)}, EMANE::Any{static_cast<long>(v7)}, EMANE::Any{static_cast<long>(v8)}, EMANE::Any{static_cast<long>(v9)}, EMANE::Any{static_cast<long>(v10)}, EMANE::Any{static_cast<long>(v11)}, EMANE::Any{static_cast<long>(v12)}, EMANE::Any{static_cast<long>(v13)}, EMANE::Any{static_cast<long>(v14)}, EMANE::Any{static_cast<long>(v15)}, EMANE::Any{static_cast<long>(v16)}, EMANE::Any{static_cast<long>(v17)}});
}

extern "C" void emane_tdma_slot_table_set_cell(void* pTable, uint32_t key, size_t column, uint64_t value) {
  auto table = static_cast<EMANE::StatisticTable<std::uint32_t>*>(pTable);
  table->setCell(key, column, EMANE::Any{static_cast<long>(value)});
}

extern "C" void emane_tdma_numeric_u64_add(void* pNumeric, uint64_t val) {
  *static_cast<EMANE::StatisticNumeric<std::uint64_t>*>(pNumeric) += val;
}

EMANE::Models::TDMA::SlotStatusTablePublisher::SlotStatusTablePublisher():
  pImpl_{emane_rs_tdma_slot_publisher_create()}
{}

EMANE::Models::TDMA::SlotStatusTablePublisher::~SlotStatusTablePublisher()
{
  emane_rs_tdma_slot_publisher_destroy(pImpl_);
}

void EMANE::Models::TDMA::SlotStatusTablePublisher::registerStatistics(StatisticRegistrar & statisticRegistrar)
{
  auto pTxSlotStatusTable =
    statisticRegistrar.registerTable<std::uint32_t>("TxSlotStatusTable",
                                                    {"Index","Frame","Slot","Valid","Missed","Big",".25",".50",".75","1.0","1.25","1.50","1.75",">1.75"},
                                                    StatisticProperties::NONE,
                                                    "Shows the number of Tx slot opportunities that were valid or missed based on slot timing deadlines");

  auto pRxSlotStatusTable =
    statisticRegistrar.registerTable<std::uint32_t>("RxSlotStatusTable",
                                                    {"Index","Frame","Slot","Valid","Missed","Idle","Tx","Long","Freq","Lock",".25",".50",".75","1.0","1.25","1.50","1.75",">1.75"},
                                                    StatisticProperties::NONE,
                                                    "Shows the number of Rx slot opportunities that were valid or missed based on slot timing deadlines");

  auto pTxSlotValid = statisticRegistrar.registerNumeric<std::uint64_t>("TxSlotValid", StatisticProperties::CLEARABLE, "Total valid Tx slots");
  auto pTxSlotErrorMissed = statisticRegistrar.registerNumeric<std::uint64_t>("TxSlotErrorMissed", StatisticProperties::CLEARABLE, "Total missed Tx slots");
  auto pTxSlotErrorTooBig = statisticRegistrar.registerNumeric<std::uint64_t>("TxSlotErrorTooBig", StatisticProperties::CLEARABLE, "Total too big Tx slots");

  auto pRxSlotValid = statisticRegistrar.registerNumeric<std::uint64_t>("RxSlotValid", StatisticProperties::CLEARABLE, "Total valid Rx slots");
  auto pRxSlotErrorMissed = statisticRegistrar.registerNumeric<std::uint64_t>("RxSlotErrorMissed", StatisticProperties::CLEARABLE, "Total missed Rx slots");
  auto pRxSlotErrorRxDuringIdle = statisticRegistrar.registerNumeric<std::uint64_t>("RxSlotErrorRxDuringIdle", StatisticProperties::CLEARABLE, "Total rx during idle Rx slots");
  auto pRxSlotErrorRxDuringTx = statisticRegistrar.registerNumeric<std::uint64_t>("RxSlotErrorRxDuringTx", StatisticProperties::CLEARABLE, "Total rx during tx Rx slots");
  auto pRxSlotErrorRxTooLong = statisticRegistrar.registerNumeric<std::uint64_t>("RxSlotErrorRxTooLong", StatisticProperties::CLEARABLE, "Total rx too long Rx slots");
  auto pRxSlotErrorRxWrongFrequency = statisticRegistrar.registerNumeric<std::uint64_t>("RxSlotErrorRxWrongFrequency", StatisticProperties::CLEARABLE, "Total rx wrong frequency Rx slots");
  auto pRxSlotErrorRxLock = statisticRegistrar.registerNumeric<std::uint64_t>("RxSlotErrorRxLock", StatisticProperties::CLEARABLE, "Total rx lock Rx slots");

  emane_rs_tdma_slot_publisher_register(pImpl_, &statisticRegistrar, pTxSlotStatusTable, pRxSlotStatusTable, pTxSlotValid, pTxSlotErrorMissed, pTxSlotErrorTooBig, pRxSlotValid, pRxSlotErrorMissed, pRxSlotErrorRxDuringIdle, pRxSlotErrorRxDuringTx, pRxSlotErrorRxTooLong, pRxSlotErrorRxWrongFrequency, pRxSlotErrorRxLock);
}

void EMANE::Models::TDMA::SlotStatusTablePublisher::update(std::uint32_t u32RelativeIndex, std::uint32_t u32RelativeFrameIndex, std::uint32_t u32RelativeSlotIndex, Status status, double dSlotRemainingRatio)
{
  emane_rs_tdma_slot_publisher_update(pImpl_, u32RelativeIndex, u32RelativeFrameIndex, u32RelativeSlotIndex, static_cast<int>(status), dSlotRemainingRatio);
}

void EMANE::Models::TDMA::SlotStatusTablePublisher::clear()
{
  emane_rs_tdma_slot_publisher_clear(pImpl_);
}
