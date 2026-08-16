import re

with open("src/libemane/tdmascheduleevent.cc", "r") as f:
    code = f.read()

# Replace msg = TDMAScheduleEvent with C structs and FFI

shim_headers = """#include "emane/events/tdmascheduleevent.h"

extern "C" {
    struct EmaneRsTdmaSlotTx {
        bool has_frequency_hz;
        uint64_t frequency_hz;
        bool has_data_rate_bps;
        uint64_t data_rate_bps;
        bool has_service_class;
        uint32_t service_class;
        bool has_power_dbm;
        double power_dbm;
        bool has_destination;
        uint32_t destination;
    };

    struct EmaneRsTdmaSlotRx {
        bool has_frequency_hz;
        uint64_t frequency_hz;
    };

    struct EmaneRsTdmaSlot {
        uint32_t index;
        int32_t type; // SLOT_TX = 1, SLOT_RX = 2, SLOT_IDLE = 3
        bool has_tx;
        EmaneRsTdmaSlotTx tx;
        bool has_rx;
        EmaneRsTdmaSlotRx rx;
    };

    struct EmaneRsTdmaFrame {
        uint32_t index;
        bool has_frequency_hz;
        uint64_t frequency_hz;
        bool has_data_rate_bps;
        uint64_t data_rate_bps;
        bool has_service_class;
        uint32_t service_class;
        bool has_power_dbm;
        double power_dbm;
        EmaneRsTdmaSlot* slots;
        size_t num_slots;
    };

    struct EmaneRsTdmaStructure {
        uint32_t slots_per_frame;
        uint32_t frames_per_multi_frame;
        uint64_t slot_duration_microseconds;
        uint64_t slot_overhead_microseconds;
        uint64_t bandwidth_hz;
    };

    struct EmaneRsTdmaSchedule {
        EmaneRsTdmaFrame* frames;
        size_t num_frames;
        bool has_structure;
        EmaneRsTdmaStructure structure;
        bool has_frequency_hz;
        uint64_t frequency_hz;
        bool has_data_rate_bps;
        uint64_t data_rate_bps;
        bool has_service_class;
        uint32_t service_class;
        bool has_power_dbm;
        double power_dbm;
    };

    bool emane_rs_tdmaschedule_event_deserialize(
        const uint8_t* buf,
        size_t len,
        EmaneRsTdmaSchedule** out_msg
    );

    void emane_rs_tdmaschedule_event_free_deserialize(EmaneRsTdmaSchedule* ptr);
}
"""

code = code.replace('#include "emane/events/tdmascheduleevent.h"\n#include "tdmascheduleevent.pb.h"', shim_headers)

parse_old = """    EMANEMessage::TDMAScheduleEvent msg{};

    if(!msg.ParseFromString(serialization))
      {
        throw SerializationException("unable to deserialize : TDMAScheduleEvent");
      }"""

parse_new = """    EmaneRsTdmaSchedule* msg_ptr = nullptr;
    if(!emane_rs_tdmaschedule_event_deserialize(
        reinterpret_cast<const uint8_t*>(serialization.c_str()),
        serialization.size(),
        &msg_ptr))
      {
        throw SerializationException("unable to deserialize : TDMAScheduleEvent");
      }
    
    // We will use a reference to make replacement easier
    const EmaneRsTdmaSchedule& msg = *msg_ptr;"""

code = code.replace(parse_old, parse_new)

code = code.replace("msg.has_structure()", "msg.has_structure")
code = code.replace("msg.structure()", "msg.structure")
code = code.replace("structure.framespermultiframe()", "structure.frames_per_multi_frame")
code = code.replace("structure.slotsperframe()", "structure.slots_per_frame")
code = code.replace("structure.bandwidthhz()", "structure.bandwidth_hz")
code = code.replace("structure.slotdurationmicroseconds()", "structure.slot_duration_microseconds")
code = code.replace("structure.slotoverheadmicroseconds()", "structure.slot_overhead_microseconds")

# For frames, we iterate over a pointer array
code = code.replace("for(const auto & frame :msg.frames())", "for(size_t _f = 0; _f < msg.num_frames; ++_f)")
code = code.replace("std::uint32_t u32FrameIndex = frame.index();", "const auto& frame = msg.frames[_f];\n        std::uint32_t u32FrameIndex = frame.index;")

# For slots
code = code.replace("for(const auto & slot : frame.slots())", "for(size_t _s = 0; _s < frame.num_slots; ++_s)")
code = code.replace("std::uint32_t u32SlotIndex = slot.index();", "const auto& slot = frame.slots[_s];\n            std::uint32_t u32SlotIndex = slot.index;")


code = code.replace("slot.type()", "slot.type")
code = code.replace("EMANEMessage::TDMAScheduleEvent::Frame::Slot::SLOT_TX", "1")
code = code.replace("EMANEMessage::TDMAScheduleEvent::Frame::Slot::SLOT_RX", "2")
code = code.replace("EMANEMessage::TDMAScheduleEvent::Frame::Slot::SLOT_IDLE", "3")

code = code.replace("slot.tx()", "slot.tx")
code = code.replace("tx.has_frequencyhz()", "tx.has_frequency_hz")
code = code.replace("tx.frequencyhz()", "tx.frequency_hz")
code = code.replace("frame.has_frequencyhz()", "frame.has_frequency_hz")
code = code.replace("frame.frequencyhz()", "frame.frequency_hz")
code = code.replace("msg.has_frequencyhz()", "msg.has_frequency_hz")
code = code.replace("msg.frequencyhz()", "msg.frequency_hz")

code = code.replace("tx.has_dataratebps()", "tx.has_data_rate_bps")
code = code.replace("tx.dataratebps()", "tx.data_rate_bps")
code = code.replace("frame.has_dataratebps()", "frame.has_data_rate_bps")
code = code.replace("frame.dataratebps()", "frame.data_rate_bps")
code = code.replace("msg.has_dataratebps()", "msg.has_data_rate_bps")
code = code.replace("msg.dataratebps()", "msg.data_rate_bps")

code = code.replace("tx.has_serviceclass()", "tx.has_service_class")
code = code.replace("tx.serviceclass()", "tx.service_class")
code = code.replace("frame.has_serviceclass()", "frame.has_service_class")
code = code.replace("frame.serviceclass()", "frame.service_class")
code = code.replace("msg.has_serviceclass()", "msg.has_service_class")
code = code.replace("msg.serviceclass()", "msg.service_class")

code = code.replace("tx.has_powerdbm()", "tx.has_power_dbm")
code = code.replace("tx.powerdbm()", "tx.power_dbm")
code = code.replace("frame.has_powerdbm()", "frame.has_power_dbm")
code = code.replace("frame.powerdbm()", "frame.power_dbm")
code = code.replace("msg.has_powerdbm()", "msg.has_power_dbm")
code = code.replace("msg.powerdbm()", "msg.power_dbm")

code = code.replace("tx.has_destination()", "tx.has_destination")
code = code.replace("tx.destination()", "tx.destination")

code = code.replace("slot.rx()", "slot.rx")
code = code.replace("rx.has_frequencyhz()", "rx.has_frequency_hz")
code = code.replace("rx.frequencyhz()", "rx.frequency_hz")

# Find the end of the constructor
code = code.replace("      }\n  }\n\n  const SlotInfos & getSlotInfos() const", "      }\n\n    emane_rs_tdmaschedule_event_free_deserialize(msg_ptr);\n  }\n\n  const SlotInfos & getSlotInfos() const")


with open("src/libemane/tdmascheduleevent.cc", "w") as f:
    f.write(code)
