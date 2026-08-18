import os

cpp_code = """
#include "frameworkphy.h"

extern "C" {
    void emane_c_framework_phy_initialize_stub(void* registrar) {}
    void emane_c_framework_phy_configure_stub(void* update) {}
    void emane_c_framework_phy_common_layer_statistics_process_inbound(void* stats, void* pkt) {}
    void emane_c_framework_phy_common_layer_statistics_process_outbound_drop(void* stats, void* pkt, uint32_t drop_code) {}
    bool emane_c_framework_phy_check_in_band(void* phy, void* header) { return false; }
    void emane_c_framework_phy_create_default_antenna_if_needed(void* phy) {}
    bool emane_c_framework_phy_receive_processors_is_empty(void* rx_procs) { return false; }
    bool emane_c_framework_phy_process_receive_processors(void* phy, void* header, void* pkt, bool in_band) { return false; }
    void emane_c_framework_phy_downstream_stub_process_inbound(void* stats, void* pkt) {}
    void emane_c_framework_phy_downstream_stub_process_outbound(void* stats, void* pkt, uint64_t duration) {}
    void emane_c_framework_phy_downstream_stub_send_downstream_packet(void* phy, void* pkt) {}
    void emane_c_framework_phy_downstream_stub_process_self_interference(void* phy, uint16_t ant_index, uint64_t now, uint64_t tx_time, void* freq_groups, uint64_t bw, double pwr, void* filter) {}
    size_t emane_c_framework_phy_downstream_stub_get_control_messages_len(void* msgs) { return 0; }
    void* emane_c_framework_phy_downstream_stub_get_control_message(void* msgs, size_t index) { return nullptr; }
    uint16_t emane_c_framework_phy_downstream_stub_get_control_message_id(void* msg) { return 0; }
}
"""

with open("scratch/frameworkphy_ffi.cc", "w") as f:
    f.write(cpp_code)
