#ifndef RUST_BENTPIPE_QUEUEMANAGER_H
#define RUST_BENTPIPE_QUEUEMANAGER_H

#include <cstdint>
#include <cstddef>
#include <stdbool.h>

extern "C" {

struct BentPipeDequeueAction {
    uint8_t action_type; // 0 = DROP, 1 = DEQUEUE_FULL, 2 = DEQUEUE_FRAGMENT
    uint16_t transponder_index;
    void* pkt_ptr;
    uint64_t seq;
    size_t fragment_index;
    size_t fragment_offset;
    size_t fragment_size;
    bool more_fragments;
};

struct BentPipeEnqueueResult {
    void* dropped_pkt;
    bool dropped;
};

struct BentPipeDequeueResult {
    BentPipeDequeueAction* actions;
    size_t num_actions;
    size_t total_bytes;
};

struct BentPipeQueueInfo {
    uint16_t transponder_index;
    size_t packets;
    size_t bytes;
};

struct BentPipeQueueInfosResult {
    BentPipeQueueInfo* infos;
    size_t num_infos;
};

void* bentpipe_queue_manager_new();
void bentpipe_queue_manager_free(void* m);
void bentpipe_queue_manager_set_config(void* m, uint16_t queue_depth, bool aggregation_enable, bool fragmentation_enable);
void bentpipe_queue_manager_add_queue(void* m, uint16_t transponder_index);
void bentpipe_queue_manager_remove_queue(void* m, uint16_t transponder_index);
BentPipeEnqueueResult bentpipe_queue_manager_enqueue(void* m, uint16_t transponder_index, void* pkt_ptr, size_t length);
void bentpipe_queue_manager_dequeue(void* m, uint16_t transponder_index, size_t requested_bytes, BentPipeDequeueResult* res);
void bentpipe_queue_manager_free_dequeue_result(BentPipeDequeueResult* res);
void bentpipe_queue_manager_get_queue_infos(const void* m, BentPipeQueueInfosResult* res);
void bentpipe_queue_manager_free_queue_infos_result(BentPipeQueueInfosResult* res);

}

#endif
