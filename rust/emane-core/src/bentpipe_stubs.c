#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#define WEAK __attribute__((weak))

WEAK uint16_t emane_bentpipe_upstreampacket_get_src(const void* pkt) { return 0; }
WEAK uint16_t emane_bentpipe_upstreampacket_get_dst(const void* pkt) { return 0; }
WEAK uint8_t emane_bentpipe_upstreampacket_get_priority(const void* pkt) { return 0; }
WEAK size_t emane_bentpipe_upstreampacket_length(const void* pkt) { return 0; }
WEAK size_t emane_bentpipe_upstreampacket_stripLengthPrefixFraming(void* pkt) { return 0; }
WEAK const void* emane_bentpipe_upstreampacket_get(const void* pkt) { return NULL; }
WEAK uint8_t emane_bentpipe_downstreampacket_get_priority(const void* pkt) { return 0; }
WEAK uint16_t emane_bentpipe_commonmacheader_get_registration_id(const void* hdr) { return 0; }
WEAK uint64_t emane_bentpipe_commonmacheader_get_sequence_number(const void* hdr) { return 0; }
WEAK void emane_bentpipe_radiomodel_drop_registration_id(void* radiomodel, uint16_t src, uint16_t dst, size_t length) {}
WEAK void emane_bentpipe_radiomodel_drop_rx_off(void* radiomodel, uint16_t src, const void* pkt_data, size_t pkt_len) {}
WEAK int32_t emane_bentpipe_radiomodel_find_transponder(void* radiomodel, const void* msgs, uint16_t* out_antenna_index, uint64_t* out_freq_hz, uint64_t* out_span, uint64_t* out_start_of_reception) { return -1; }
WEAK void emane_bentpipe_radiomodel_enqueue_receive(void* radiomodel, int32_t transponder_index, void* pkt, const void* msgs, uint64_t seq_num) {}
WEAK int32_t emane_bentpipe_radiomodel_get_tos_to_transponder(void* radiomodel, uint8_t tos) { return -1; }
WEAK void emane_bentpipe_radiomodel_queue_manager_enqueue(void* radiomodel, int32_t transponder_index, void* pkt) {}
WEAK void emane_bentpipe_radiomodel_process(void* radiomodel) {}

WEAK uint64_t emane_bentpipe_receivemanager_cxx_now(void* cpp_this) { return 0; }
WEAK void emane_bentpipe_receivemanager_cxx_drop_lock(void* cpp_this, void* pkt_info_ptr, void* msg_ptr) {}
WEAK void emane_bentpipe_receivemanager_cxx_drop_spectrum_service(void* cpp_this, void* pkt_info_ptr, void* msg_ptr) {}
WEAK void emane_bentpipe_receivemanager_cxx_drop_sinr(void* cpp_this, void* pkt_info_ptr, void* msg_ptr) {}
WEAK void emane_bentpipe_receivemanager_cxx_drop_bad_curve(void* cpp_this, void* pkt_info_ptr, void* msg_ptr) {}
WEAK void emane_bentpipe_receivemanager_cxx_publish_drop_destination_mac(void* cpp_this, void* pkt_info_ptr, void* msg_ptr, size_t msg_index) {}
WEAK void emane_bentpipe_receivemanager_cxx_publish_accept_good(void* cpp_this, void* pkt_info_ptr, void* msg_ptr, size_t msg_index) {}
WEAK void emane_bentpipe_receivemanager_cxx_publish_accept_good_len(void* cpp_this, void* pkt_info_ptr, uint16_t dst, size_t length) {}
WEAK int emane_bentpipe_receivemanager_cxx_check_spectrum(void* cpp_this, uint16_t rx_antenna_index, uint64_t freq_hz, uint64_t span, uint64_t sor, double rx_power_dbm, double* out_noise_floor, bool* out_signal_in_noise) { return 0; }
WEAK int emane_bentpipe_receivemanager_cxx_get_por(void* cpp_this, uint16_t pcr_curve_index, double sinr, size_t length, float* out_por) { return 0; }
WEAK float emane_bentpipe_receivemanager_cxx_get_random(void* cpp_this) { return 0.0f; }
WEAK void emane_bentpipe_receivemanager_cxx_update_neighbor_metrics(void* cpp_this, void* pkt_info_ptr, uint16_t transponder_index, double sinr, double noise_floor, uint64_t sor) {}
WEAK size_t emane_bentpipe_receivemanager_cxx_get_messages_count(void* msg_ptr) { return 0; }
WEAK uint16_t emane_bentpipe_receivemanager_cxx_msg_get_dst(void* msg_ptr, size_t index) { return 0; }
WEAK bool emane_bentpipe_receivemanager_cxx_msg_is_fragment(void* msg_ptr, size_t index) { return false; }
WEAK size_t emane_bentpipe_receivemanager_cxx_msg_get_fragment_index(void* msg_ptr, size_t index) { return 0; }
WEAK size_t emane_bentpipe_receivemanager_cxx_msg_get_fragment_offset(void* msg_ptr, size_t index) { return 0; }
WEAK uint64_t emane_bentpipe_receivemanager_cxx_msg_get_fragment_sequence(void* msg_ptr, size_t index) { return 0; }
WEAK bool emane_bentpipe_receivemanager_cxx_msg_is_more_fragments(void* msg_ptr, size_t index) { return false; }
WEAK const uint8_t* emane_bentpipe_receivemanager_cxx_msg_get_data(void* msg_ptr, size_t index, size_t* out_len) { *out_len = 0; return NULL; }
WEAK uint16_t emane_bentpipe_receivemanager_cxx_pktinfo_get_src(void* pkt_info_ptr) { return 0; }
WEAK uint16_t emane_bentpipe_receivemanager_cxx_pktinfo_get_dst(void* pkt_info_ptr) { return 0; }
WEAK uint8_t emane_bentpipe_receivemanager_cxx_pktinfo_get_priority(void* pkt_info_ptr) { return 0; }
WEAK uint64_t emane_bentpipe_receivemanager_cxx_pktinfo_get_creation_time(void* pkt_info_ptr) { return 0; }
WEAK uint64_t emane_bentpipe_receivemanager_cxx_pktinfo_get_uuid(void* pkt_info_ptr) { return 0; }
WEAK uint16_t emane_bentpipe_receivemanager_cxx_bpm_get_pcr_curve_index(void* msg_ptr) { return 0; }
WEAK uint64_t emane_bentpipe_receivemanager_cxx_freq_get_frequency_hz(void* freq_ptr) { return 0; }
WEAK double emane_bentpipe_receivemanager_cxx_freq_get_rx_power_dbm(void* freq_ptr) { return 0.0; }
WEAK void emane_bentpipe_receivemanager_cxx_forward_upstream(void* cpp_this, void* pkt_info_ptr, uint16_t dst, const uint8_t* data, size_t len) {}
WEAK void emane_bentpipe_receivemanager_cxx_bend_downstream(void* cpp_this, void* pkt_info_ptr, uint16_t dst, const uint8_t* data, size_t len) {}
WEAK void emane_bentpipe_receivemanager_cxx_forward_upstream_parts(void* cpp_this, void* pkt_info_ptr, uint16_t dst, const uint8_t** data_ptrs, const size_t* data_lens, size_t num_parts) {}
WEAK void emane_bentpipe_receivemanager_cxx_bend_downstream_parts(void* cpp_this, void* pkt_info_ptr, uint16_t dst, const uint8_t** data_ptrs, const size_t* data_lens, size_t num_parts) {}
WEAK void emane_bentpipe_receivemanager_cxx_schedule_process(void* cpp_this, uint64_t eor) {}
WEAK void emane_bentpipe_receivemanager_cxx_drop_miss_fragment(void* cpp_this, uint16_t src, uint16_t dst, size_t total_bytes) {}
WEAK void emane_bentpipe_receivemanager_cxx_delete_msg(void* ptr) {}
WEAK void emane_bentpipe_receivemanager_cxx_delete_pkt_info(void* ptr) {}
WEAK void emane_bentpipe_receivemanager_cxx_delete_freq_segments(void* ptr) {}
