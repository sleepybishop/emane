use emane_plugin_api::{
    CommonLayerCounters, FfiConfigRequest, FfiControlMessage, FfiFrameworkService, FfiPacket,
    FfiSlice, ModelHeader, PluginApi, CONTROL_MODEL_HEADER, MAC_REGISTRATION_BYPASS,
    PLUGIN_ABI_VERSION,
};
use std::ffi::c_void;
use std::sync::OnceLock;
use std::time::Instant;

fn elapsed_microseconds(begin: Instant) -> u64 {
    begin.elapsed().as_micros().min(u128::from(u64::MAX)) as u64
}

struct BypassMac {
    id: u16,
    framework: FfiFrameworkService,
    counters: CommonLayerCounters,
    sequence: u64,
}

fn controls(
    messages: *const FfiControlMessage,
    count: usize,
) -> Option<&'static [FfiControlMessage]> {
    if count > 4096 || (count != 0 && messages.is_null()) {
        None
    } else if count == 0 {
        Some(&[])
    } else {
        Some(unsafe { std::slice::from_raw_parts(messages, count) })
    }
}

extern "C" fn init(id: u16, framework: *const FfiFrameworkService) -> *mut c_void {
    let Some(framework) = (unsafe { framework.as_ref() }) else {
        return std::ptr::null_mut();
    };
    let framework = *framework;
    Box::into_raw(Box::new(BypassMac {
        id,
        framework,
        counters: CommonLayerCounters::register_with_drop_labels(framework, "", &["Reg Id"], &[]),
        sequence: 0,
    }))
    .cast()
}

extern "C" fn configure(_: *mut c_void, request: *const c_void) -> bool {
    let Some(request) = (unsafe { (request as *const FfiConfigRequest).as_ref() }) else {
        return false;
    };
    request.len == 0
}

extern "C" fn start(plugin: *mut c_void) -> bool {
    !plugin.is_null()
}

extern "C" fn lifecycle(_: *mut c_void) {}

extern "C" fn destroy(plugin: *mut c_void) {
    if !plugin.is_null() {
        unsafe { drop(Box::from_raw(plugin as *mut BypassMac)) };
    }
}

extern "C" fn upstream(
    plugin: *mut c_void,
    packet: *const FfiPacket,
    messages: *const FfiControlMessage,
    count: usize,
) {
    let Some(mac) = (unsafe { (plugin as *mut BypassMac).as_ref() }) else {
        return;
    };
    let begin = Instant::now();
    if packet.is_null() {
        (mac.framework.send_upstream_control)(mac.framework.framework_ctx, mac.id, messages, count);
        return;
    }
    let Some(messages) = controls(messages, count) else {
        return;
    };
    let packet_ref = unsafe { &*packet };
    mac.counters.upstream_rx_packet(
        mac.framework,
        packet_ref.info.source,
        packet_ref.info.destination,
        packet_ref.payload.len,
    );
    let valid = messages.iter().any(|message| {
        message.msg_type == CONTROL_MODEL_HEADER
            && message.payload.len == ModelHeader::ENCODED_LEN
            && !message.payload.data.is_null()
            && ModelHeader::decode(unsafe {
                std::slice::from_raw_parts(message.payload.data, message.payload.len)
            })
            .is_some_and(|header| header.registration_id == MAC_REGISTRATION_BYPASS)
    });
    if !valid {
        mac.counters.upstream_drop_packet(
            mac.framework,
            packet_ref.info.source,
            packet_ref.info.destination,
            1,
        );
        return;
    }
    let outgoing: Vec<_> = messages
        .iter()
        .copied()
        .filter(|message| message.msg_type != CONTROL_MODEL_HEADER)
        .collect();
    mac.counters.upstream_tx_packet(
        mac.framework,
        packet_ref.info.source,
        packet_ref.info.destination,
        packet_ref.payload.len,
        elapsed_microseconds(begin),
    );
    (mac.framework.send_upstream_packet)(
        mac.framework.framework_ctx,
        mac.id,
        packet,
        outgoing.as_ptr(),
        outgoing.len(),
    );
}

extern "C" fn downstream(
    plugin: *mut c_void,
    packet: *const FfiPacket,
    messages: *const FfiControlMessage,
    count: usize,
) {
    let Some(mac) = (unsafe { (plugin as *mut BypassMac).as_mut() }) else {
        return;
    };
    let begin = Instant::now();
    if packet.is_null() {
        (mac.framework.send_downstream_control)(
            mac.framework.framework_ctx,
            mac.id,
            messages,
            count,
        );
        return;
    }
    let Some(messages) = controls(messages, count) else {
        return;
    };
    let packet_ref = unsafe { &*packet };
    mac.counters.downstream_rx_packet(
        mac.framework,
        packet_ref.info.source,
        packet_ref.info.destination,
        packet_ref.payload.len,
    );
    let header = ModelHeader {
        registration_id: MAC_REGISTRATION_BYPASS,
        sequence: mac.sequence,
        data_rate_bps: 0,
        category: 0,
        message_type: 0,
        flags: 0,
    };
    mac.sequence = mac.sequence.wrapping_add(1);
    let header = header.encode();
    let mut outgoing: Vec<_> = messages
        .iter()
        .copied()
        .filter(|message| message.msg_type != CONTROL_MODEL_HEADER)
        .collect();
    outgoing.push(FfiControlMessage {
        msg_type: CONTROL_MODEL_HEADER,
        payload: FfiSlice {
            data: header.as_ptr(),
            len: header.len(),
        },
    });
    mac.counters.downstream_tx_packet(
        mac.framework,
        packet_ref.info.source,
        packet_ref.info.destination,
        packet_ref.payload.len,
        elapsed_microseconds(begin),
        false,
    );
    (mac.framework.send_downstream_packet)(
        mac.framework.framework_ctx,
        mac.id,
        packet,
        outgoing.as_ptr(),
        outgoing.len(),
    );
}

extern "C" fn timed(_: *mut c_void, _: u64, _: u32, _: *const u8, _: usize) {}
extern "C" fn event(_: *mut c_void, _: u16, _: *const u8, _: usize) {}

#[no_mangle]
pub extern "C" fn emane_plugin_create() -> *const PluginApi {
    static API: OnceLock<PluginApi> = OnceLock::new();
    API.get_or_init(|| PluginApi {
        abi_version: PLUGIN_ABI_VERSION,
        struct_size: std::mem::size_of::<PluginApi>(),
        name: c"bypassmaclayer".as_ptr(),
        plugin_type: 1,
        init,
        configure,
        start,
        post_start: lifecycle,
        stop: lifecycle,
        destroy,
        process_upstream: upstream,
        process_downstream: downstream,
        process_timed_event: timed,
        process_event: event,
    })
}
