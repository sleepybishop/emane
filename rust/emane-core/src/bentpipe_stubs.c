#include <stdint.h>
#include <stddef.h>

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
