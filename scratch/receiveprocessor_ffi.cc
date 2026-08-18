#include "receiveprocessor.h"
#include <vector>
#include <map>
#include <tuple>

extern "C" {
    void emane_c_receive_processor_add_receive_power(
        void* result_ptr,
        uint16_t src, uint16_t rx_ant, uint16_t tx_ant, uint64_t freq,
        double rx_power, double tx_gain, double rx_gain, double tx_power, double pathloss, double doppler
    ) {
        auto res = static_cast<EMANE::ReceiveProcessor::ProcessResult*>(result_ptr);
        res->receivePowerMap_.insert(std::make_pair(
            std::make_tuple(src, rx_ant, tx_ant, freq),
            std::make_tuple(rx_power, tx_gain, rx_gain, tx_power, pathloss, doppler)
        ));
    }

    void emane_c_receive_processor_add_observed_power(
        void* result_ptr,
        uint16_t src, uint16_t rx_ant, uint16_t tx_ant, uint64_t freq,
        uint16_t mask_index, double rx_power
    ) {
        auto res = static_cast<EMANE::ReceiveProcessor::ProcessResult*>(result_ptr);
        res->observedPowerMap_.insert(std::make_pair(
            std::make_tuple(src, rx_ant, tx_ant, freq),
            std::make_tuple(mask_index, rx_power)
        ));
    }
}
