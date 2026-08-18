#include <cstdint>
/*
 * Copyright (c) 2022-2023 - Adjacent Link LLC, Bridgewater, New Jersey
 * All rights reserved.
 */

#include "emane/rfsignaltable.h"
#include "emane/statistictable.h"
#include <vector>

extern "C" {
    struct RfSignalUpdateResult {
        bool is_new;
        char* key;
        uint16_t src;
        uint16_t rx_antenna_id;
        uint64_t frequency_hz;
        uint64_t num_samples;
        double avg_rx_power;
        double avg_noise_floor;
        double avg_sinr;
        double avg_inr;
    };

    void* emane_rs_rf_signal_table_new(uint16_t nem_id);
    void emane_rs_rf_signal_table_free(void* ptr);
    void emane_rs_rf_signal_table_configure(void* ptr, bool b_average_all_antenna, bool b_average_all_frequencies);
    void emane_rs_rf_signal_table_update(
        void* ptr,
        uint16_t src,
        uint16_t rx_antenna_id,
        uint64_t frequency_hz,
        double rx_power_dbm,
        double sinr_db,
        double noise_floor_db,
        double receiver_sensitivity_db,
        RfSignalUpdateResult* out_result
    );
    void emane_rs_rf_signal_table_free_string(char* ptr);
    char** emane_rs_rf_signal_table_reset(void* ptr, uint16_t rx_antenna_id, size_t* out_keys_len);
    void emane_rs_rf_signal_table_free_keys(char** keys_ptr, size_t len);
    void emane_rs_rf_signal_table_reset_all(void* ptr);
}

enum {
  RFSIGNALTABLE_NEMID,
  RFSIGNALTABLE_ANTENNAID,
  RFSIGNALTABLE_FREQUENCY,
  RFSIGNALTABLE_NUM_SAMPLES,
  RFSIGNALTABLE_AVG_RX_POWER,
  RFSIGNALTABLE_AVG_NOISE_FLOOR,
  RFSIGNALTABLE_AVG_SINR,
  RFSIGNALTABLE_AVG_INR
};

class EMANE::RFSignalTable::Implementation
{
public:
  Implementation(NEMId nemId) :
    nemId_{nemId},
    pStatisticRFSignalTable_{},
    bAverageAllAntenna_{false},
    bAverageAllFrequencies_{false},
    rs_state_{emane_rs_rf_signal_table_new(nemId)}
  { }

  ~Implementation()
  {
    emane_rs_rf_signal_table_free(rs_state_);
  }

  void configure(const ConfigurationUpdate & update)
  {
    for(const auto & item : update)
      {
        if(item.first == CONFIG_PREFIX + std::string("averageallantenna"))
          {
            bAverageAllAntenna_ = item.second[0].asBool();
          }
        else if(item.first == CONFIG_PREFIX + std::string("averageallfrequencies"))
          {
            bAverageAllFrequencies_ = item.second[0].asBool();
          }
      }
    emane_rs_rf_signal_table_configure(rs_state_, bAverageAllAntenna_, bAverageAllFrequencies_);
  }

  void initialize(Registrar & registrar)
  {
    auto & statisticRegistrar = registrar.statisticRegistrar();
    auto & configurationRegistrar = registrar.configurationRegistrar();

    configurationRegistrar.registerNumeric<bool>(CONFIG_PREFIX + std::string("averageallantenna"),
                                                 ConfigurationProperties::DEFAULT |
                                                 ConfigurationProperties::MODIFIABLE,
                                                 {false},
                                                 "Average receive metrics over all antennas.");

    configurationRegistrar.registerNumeric<bool>(CONFIG_PREFIX + std::string("averageallfrequencies"),
                                                 ConfigurationProperties::DEFAULT |
                                                 ConfigurationProperties::MODIFIABLE,
                                                 {false},
                                                 "Average receive metrics over all frequencies.");

    pStatisticRFSignalTable_ =
      statisticRegistrar.registerTable<std::string>("ReceiveMetricTable",
                                                    {"NEM",
                                                     "Antenna",
                                                     "Frequency",
                                                     "Samples",
                                                     "Avg Rx Power",
                                                     "Avg Noise",
                                                     "Avg SINR",
                                                     "Avg INR"},
                                                    StatisticProperties::CLEARABLE,
                                                    "Table of RF receive metrics from peering NEMs");

    tableRowTemplate_.push_back(Any{std::uint64_t{}});
    tableRowTemplate_.push_back(Any{std::string{"NA"}});
    tableRowTemplate_.push_back(Any{std::string{"NA"}});
    tableRowTemplate_.push_back(Any{std::uint64_t{}});
    tableRowTemplate_.push_back(Any{double{}});
    tableRowTemplate_.push_back(Any{double{}});
    tableRowTemplate_.push_back(Any{double{}});
    tableRowTemplate_.push_back(Any{double{}});
  }

  void reset(AntennaIndex rxAntennaId)
  {
    size_t keys_len = 0;
    char** keys = emane_rs_rf_signal_table_reset(rs_state_, rxAntennaId, &keys_len);
    for (size_t i = 0; i < keys_len; ++i) {
        std::string key_str(keys[i]);
        if (pStatisticRFSignalTable_) {
            pStatisticRFSignalTable_->deleteRow(key_str);
        }
    }
    emane_rs_rf_signal_table_free_keys(keys, keys_len);
  }

  void resetAll()
  {
    if (pStatisticRFSignalTable_) {
        pStatisticRFSignalTable_->clear();
    }
    emane_rs_rf_signal_table_reset_all(rs_state_);
  }

  void update(NEMId src,
              AntennaIndex rxAntennaId,
              std::uint64_t frequencyHz,
              double dRxPower_dBm,
              double dSINR_dB,
              double dNoiseFloor_dB,
              double dReceiverSensitivity_dB)
  {
    if(! pStatisticRFSignalTable_)
      {
        return;
      }

    RfSignalUpdateResult res;
    emane_rs_rf_signal_table_update(rs_state_, src, rxAntennaId, frequencyHz, dRxPower_dBm, dSINR_dB, dNoiseFloor_dB, dReceiverSensitivity_dB, &res);

    std::string key(res.key);
    emane_rs_rf_signal_table_free_string(res.key);

    if (res.is_new) {
        pStatisticRFSignalTable_->addRow(key, tableRowTemplate_);
        pStatisticRFSignalTable_->setCell(key, RFSIGNALTABLE_NEMID, Any{res.src});

        if(! bAverageAllAntenna_)
          {
            pStatisticRFSignalTable_->setCell(key, RFSIGNALTABLE_ANTENNAID, Any{res.rx_antenna_id});
          }

        if(! bAverageAllFrequencies_)
          {
            pStatisticRFSignalTable_->setCell(key, RFSIGNALTABLE_FREQUENCY,  Any{res.frequency_hz});
          }
    }

    pStatisticRFSignalTable_->setCell(key, RFSIGNALTABLE_NUM_SAMPLES,     Any{res.num_samples});
    pStatisticRFSignalTable_->setCell(key, RFSIGNALTABLE_AVG_RX_POWER,    Any{res.avg_rx_power});
    pStatisticRFSignalTable_->setCell(key, RFSIGNALTABLE_AVG_NOISE_FLOOR, Any{res.avg_noise_floor});
    pStatisticRFSignalTable_->setCell(key, RFSIGNALTABLE_AVG_SINR,        Any{res.avg_sinr});
    pStatisticRFSignalTable_->setCell(key, RFSIGNALTABLE_AVG_INR ,        Any{res.avg_inr});
  }

private:
  NEMId nemId_;
  StatisticTable<std::string> * pStatisticRFSignalTable_;
  bool bAverageAllAntenna_;
  bool bAverageAllFrequencies_;
  std::vector<Any> tableRowTemplate_;
  void* rs_state_;
};

EMANE::RFSignalTable::RFSignalTable(NEMId nemId) :
  pImpl_{new Implementation{nemId}}
{ }

EMANE::RFSignalTable::~RFSignalTable()
{ }

void EMANE::RFSignalTable::configure(const ConfigurationUpdate & configurationUpdate)
{
  pImpl_->configure(configurationUpdate);
}

void EMANE::RFSignalTable::initialize(Registrar & registrar)
{
  pImpl_->initialize(registrar);
}

void EMANE::RFSignalTable::reset(AntennaIndex rxAntennaId)
{
  pImpl_->reset(rxAntennaId);
}

void EMANE::RFSignalTable::resetAll()
{
  pImpl_->resetAll();
}

void
EMANE::RFSignalTable::update(NEMId src,
                             AntennaIndex rxAntennaId,
                             std::uint64_t frequencyHz,
                             double dRxPower_dBm,
                             double dSINR_dB,
                             double dNoiseFloor_dB,
                             double dReceiverSensitivity_dB)
{
  pImpl_->update(src,
                 rxAntennaId,
                 frequencyHz,
                 dRxPower_dBm,
                 dSINR_dB,
                 dNoiseFloor_dB,
                 dReceiverSensitivity_dB);
}
