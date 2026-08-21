use std::time::SystemTime;

pub type NemId = u16;

#[derive(Clone, Debug)]
pub struct PacketInfo {
    pub source: NemId,
    pub destination: NemId,
    pub priority: u8,
    pub creation_time: SystemTime,
}

#[derive(Clone, Debug)]
pub struct ControlMessage {
    pub msg_type: u32,
    pub data: Vec<u8>,
}

pub type ControlMessages = Vec<ControlMessage>;

#[derive(Clone, Debug)]
pub struct UpstreamPacket {
    pub info: PacketInfo,
    pub payload: Vec<u8>,
}

#[derive(Clone, Debug)]
pub struct DownstreamPacket {
    pub info: PacketInfo,
    pub payload: Vec<u8>,
}
