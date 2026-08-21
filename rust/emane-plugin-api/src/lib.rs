use std::os::raw::{c_char, c_void};

/// Native control-message identifiers used at the Rust MAC/PHY boundary.
/// Values live outside the legacy framework range and the payloads use an
/// explicitly versioned, big-endian wire format so they are also safe to
/// carry over OTA multicast.
pub const CONTROL_TX_PROPERTIES: u32 = 0x454D_0001;
pub const CONTROL_RX_PROPERTIES: u32 = 0x454D_0002;
pub const CONTROL_MODEL_HEADER: u32 = 0x454D_0003;
pub const CONTROL_FREQUENCY_INTEREST: u32 = 0x454D_0004;

pub const MAC_REGISTRATION_IEEE80211ABG: u16 = 0x0003;
pub const MAC_REGISTRATION_RFPIPE: u16 = 0x0004;
pub const MAC_REGISTRATION_TDMA: u16 = 0x0005;
pub const MAC_REGISTRATION_BENTPIPE: u16 = 0x0006;

const PROPERTIES_VERSION: u8 = 1;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrequencyOfInterest {
    pub bandwidth_hz: u64,
    pub frequencies_hz: Vec<u64>,
}

impl FrequencyOfInterest {
    pub fn encode(&self) -> Option<Vec<u8>> {
        let count = u16::try_from(self.frequencies_hz.len()).ok()?;
        if self.bandwidth_hz == 0 || count == 0 || self.frequencies_hz.contains(&0) {
            return None;
        }
        let mut data = Vec::with_capacity(11 + self.frequencies_hz.len() * 8);
        data.push(PROPERTIES_VERSION);
        data.extend_from_slice(&count.to_be_bytes());
        data.extend_from_slice(&self.bandwidth_hz.to_be_bytes());
        for frequency in &self.frequencies_hz {
            data.extend_from_slice(&frequency.to_be_bytes());
        }
        Some(data)
    }

    pub fn decode(data: &[u8]) -> Option<Self> {
        if data.len() < 19 || data[0] != PROPERTIES_VERSION {
            return None;
        }
        let count = get_u16(data, 1) as usize;
        if count == 0 || data.len() != 11usize.checked_add(count.checked_mul(8)?)? {
            return None;
        }
        let bandwidth_hz = get_u64(data, 3);
        let frequencies_hz: Vec<_> = (0..count)
            .map(|index| get_u64(data, 11 + index * 8))
            .collect();
        (bandwidth_hz != 0 && !frequencies_hz.contains(&0)).then_some(Self {
            bandwidth_hz,
            frequencies_hz,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ModelHeader {
    pub registration_id: u16,
    pub sequence: u64,
    pub data_rate_bps: u64,
    pub category: u8,
    pub message_type: u8,
    pub flags: u16,
}

impl ModelHeader {
    pub const ENCODED_LEN: usize = 24;

    pub fn encode(self) -> [u8; Self::ENCODED_LEN] {
        let mut data = [0u8; Self::ENCODED_LEN];
        data[0] = PROPERTIES_VERSION;
        put_u16(&mut data, 1, self.registration_id);
        put_u64(&mut data, 3, self.sequence);
        put_u64(&mut data, 11, self.data_rate_bps);
        data[19] = self.category;
        data[20] = self.message_type;
        put_u16(&mut data, 21, self.flags);
        data
    }

    pub fn decode(data: &[u8]) -> Option<Self> {
        (data.len() >= Self::ENCODED_LEN && data[0] == PROPERTIES_VERSION).then(|| Self {
            registration_id: get_u16(data, 1),
            sequence: get_u64(data, 3),
            data_rate_bps: get_u64(data, 11),
            category: data[19],
            message_type: data[20],
            flags: get_u16(data, 21),
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TxProperties {
    pub frequency_hz: u64,
    pub bandwidth_hz: u64,
    pub tx_power_dbm: f64,
    pub duration_microseconds: u64,
    pub offset_microseconds: u64,
    pub tx_time_microseconds: i64,
    pub antenna_index: u16,
    pub spectral_mask_index: u16,
    pub sub_id: u16,
}

impl TxProperties {
    pub const ENCODED_LEN: usize = 55;

    pub fn encode(self) -> [u8; Self::ENCODED_LEN] {
        let mut data = [0u8; Self::ENCODED_LEN];
        data[0] = PROPERTIES_VERSION;
        put_u64(&mut data, 1, self.frequency_hz);
        put_u64(&mut data, 9, self.bandwidth_hz);
        put_u64(&mut data, 17, self.tx_power_dbm.to_bits());
        put_u64(&mut data, 25, self.duration_microseconds);
        put_u64(&mut data, 33, self.offset_microseconds);
        put_u64(&mut data, 41, self.tx_time_microseconds as u64);
        put_u16(&mut data, 49, self.antenna_index);
        put_u16(&mut data, 51, self.spectral_mask_index);
        put_u16(&mut data, 53, self.sub_id);
        data
    }

    pub fn decode(data: &[u8]) -> Option<Self> {
        (data.len() == Self::ENCODED_LEN && data[0] == PROPERTIES_VERSION).then(|| Self {
            frequency_hz: get_u64(data, 1),
            bandwidth_hz: get_u64(data, 9),
            tx_power_dbm: f64::from_bits(get_u64(data, 17)),
            duration_microseconds: get_u64(data, 25),
            offset_microseconds: get_u64(data, 33),
            tx_time_microseconds: get_u64(data, 41) as i64,
            antenna_index: get_u16(data, 49),
            spectral_mask_index: get_u16(data, 51),
            sub_id: get_u16(data, 53),
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RxProperties {
    pub frequency_hz: u64,
    pub bandwidth_hz: u64,
    pub rx_power_dbm: f64,
    pub noise_floor_dbm: f64,
    pub tx_time_microseconds: i64,
    pub propagation_microseconds: u64,
    pub duration_microseconds: u64,
    pub antenna_index: u16,
    pub sub_id: u16,
    pub signal_in_noise: bool,
}

impl RxProperties {
    pub const ENCODED_LEN: usize = 63;

    pub fn encode(self) -> [u8; Self::ENCODED_LEN] {
        let mut data = [0u8; Self::ENCODED_LEN];
        data[0] = PROPERTIES_VERSION;
        put_u64(&mut data, 1, self.frequency_hz);
        put_u64(&mut data, 9, self.bandwidth_hz);
        put_u64(&mut data, 17, self.rx_power_dbm.to_bits());
        put_u64(&mut data, 25, self.noise_floor_dbm.to_bits());
        put_u64(&mut data, 33, self.tx_time_microseconds as u64);
        put_u64(&mut data, 41, self.propagation_microseconds);
        put_u64(&mut data, 49, self.duration_microseconds);
        put_u16(&mut data, 57, self.antenna_index);
        put_u16(&mut data, 59, self.sub_id);
        data[61] = u8::from(self.signal_in_noise);
        // Byte 62 is reserved for compatible extension.
        data
    }

    pub fn decode(data: &[u8]) -> Option<Self> {
        (data.len() == Self::ENCODED_LEN && data[0] == PROPERTIES_VERSION && data[61] <= 1).then(
            || Self {
                frequency_hz: get_u64(data, 1),
                bandwidth_hz: get_u64(data, 9),
                rx_power_dbm: f64::from_bits(get_u64(data, 17)),
                noise_floor_dbm: f64::from_bits(get_u64(data, 25)),
                tx_time_microseconds: get_u64(data, 33) as i64,
                propagation_microseconds: get_u64(data, 41),
                duration_microseconds: get_u64(data, 49),
                antenna_index: get_u16(data, 57),
                sub_id: get_u16(data, 59),
                signal_in_noise: data[61] != 0,
            },
        )
    }
}

fn put_u16(data: &mut [u8], offset: usize, value: u16) {
    data[offset..offset + 2].copy_from_slice(&value.to_be_bytes());
}

fn put_u64(data: &mut [u8], offset: usize, value: u64) {
    data[offset..offset + 8].copy_from_slice(&value.to_be_bytes());
}

fn get_u16(data: &[u8], offset: usize) -> u16 {
    u16::from_be_bytes(data[offset..offset + 2].try_into().unwrap())
}

fn get_u64(data: &[u8], offset: usize) -> u64 {
    u64::from_be_bytes(data[offset..offset + 8].try_into().unwrap())
}

// Version 2 made configuration and start failures explicit. Version 3 adds
// framework event delivery; events must not be disguised as timer callbacks
// because timer identifiers are plugin-owned cancellation tokens.
pub const PLUGIN_ABI_VERSION: u32 = 3;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct FfiSlice {
    pub data: *const u8,
    pub len: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct FfiPacketInfo {
    pub source: u16,
    pub destination: u16,
    pub priority: u8,
    pub creation_time_sec: u64,
    pub creation_time_usec: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct FfiControlMessage {
    pub msg_type: u32,
    pub payload: FfiSlice,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct FfiPacket {
    pub info: FfiPacketInfo,
    pub payload: FfiSlice,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct FfiFrameworkService {
    pub framework_ctx: *mut c_void,
    pub send_downstream_packet: extern "C" fn(
        ctx: *mut c_void,
        nem_id: u16,
        packet: *const FfiPacket,
        messages: *const FfiControlMessage,
        message_count: usize,
    ),
    pub send_upstream_packet: extern "C" fn(
        ctx: *mut c_void,
        nem_id: u16,
        packet: *const FfiPacket,
        messages: *const FfiControlMessage,
        message_count: usize,
    ),
    pub send_downstream_control: extern "C" fn(
        ctx: *mut c_void,
        nem_id: u16,
        messages: *const FfiControlMessage,
        message_count: usize,
    ),
    pub send_upstream_control: extern "C" fn(
        ctx: *mut c_void,
        nem_id: u16,
        messages: *const FfiControlMessage,
        message_count: usize,
    ),
    pub schedule_timed_event: extern "C" fn(
        ctx: *mut c_void,
        nem_id: u16,
        time_sec: u64,
        time_usec: u32,
        event_id: u32,
        data: *const u8,
        data_len: usize,
    ) -> u64,
    pub cancel_timed_event: extern "C" fn(ctx: *mut c_void, nem_id: u16, timer_id: u64),
    pub log: extern "C" fn(ctx: *mut c_void, level: u32, message: *const c_char),
}

unsafe impl Send for FfiFrameworkService {}
unsafe impl Sync for FfiFrameworkService {}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct FfiConfigStringArray {
    pub data: *const *const c_char,
    pub len: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct FfiConfigItem {
    pub name: *const c_char,
    pub values: FfiConfigStringArray,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct FfiConfigRequest {
    pub data: *const FfiConfigItem,
    pub len: usize,
}

#[repr(C)]
pub struct PluginApi {
    pub abi_version: u32,
    pub struct_size: usize,
    pub name: *const c_char,
    pub plugin_type: u32,
    pub init: extern "C" fn(id: u16, framework: *const FfiFrameworkService) -> *mut c_void,
    pub configure: extern "C" fn(plugin: *mut c_void, request: *const c_void) -> bool,
    pub start: extern "C" fn(plugin: *mut c_void) -> bool,
    pub post_start: extern "C" fn(plugin: *mut c_void),
    pub stop: extern "C" fn(plugin: *mut c_void),
    pub destroy: extern "C" fn(plugin: *mut c_void),
    pub process_upstream: extern "C" fn(
        plugin: *mut c_void,
        packet: *const FfiPacket,
        messages: *const FfiControlMessage,
        message_count: usize,
    ),
    pub process_downstream: extern "C" fn(
        plugin: *mut c_void,
        packet: *const FfiPacket,
        messages: *const FfiControlMessage,
        message_count: usize,
    ),
    pub process_timed_event: extern "C" fn(
        plugin: *mut c_void,
        timer_id: u64,
        event_id: u32,
        data: *const u8,
        data_len: usize,
    ),
    pub process_event:
        extern "C" fn(plugin: *mut c_void, event_id: u16, data: *const u8, data_len: usize),
}

unsafe impl Send for PluginApi {}
unsafe impl Sync for PluginApi {}

pub type PluginEntryFunc = extern "C" fn() -> *const PluginApi;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn properties_wire_formats_round_trip() {
        let tx = TxProperties {
            frequency_hz: 2_347_000_000,
            bandwidth_hz: 1_000_000,
            tx_power_dbm: -3.25,
            duration_microseconds: 42,
            offset_microseconds: 7,
            tx_time_microseconds: -8,
            antenna_index: 2,
            spectral_mask_index: 4,
            sub_id: 9,
        };
        assert_eq!(TxProperties::decode(&tx.encode()), Some(tx));

        let rx = RxProperties {
            frequency_hz: tx.frequency_hz,
            bandwidth_hz: tx.bandwidth_hz,
            rx_power_dbm: -44.0,
            noise_floor_dbm: -101.5,
            tx_time_microseconds: 88,
            propagation_microseconds: 2,
            duration_microseconds: 42,
            antenna_index: 3,
            sub_id: 9,
            signal_in_noise: true,
        };
        assert_eq!(RxProperties::decode(&rx.encode()), Some(rx));

        let header = ModelHeader {
            registration_id: MAC_REGISTRATION_RFPIPE,
            sequence: 44,
            data_rate_bps: 1_000_000,
            category: 3,
            message_type: 2,
            flags: 9,
        };
        assert_eq!(ModelHeader::decode(&header.encode()), Some(header));

        let foi = FrequencyOfInterest {
            bandwidth_hz: 2_000_000,
            frequencies_hz: vec![100, 200],
        };
        assert_eq!(
            FrequencyOfInterest::decode(&foi.encode().unwrap()),
            Some(foi)
        );
    }
}
