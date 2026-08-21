use crate::protobufs::emane_message::RfPipeMacHeader;
use prost::Message as ProstMessage;

pub struct RFPipeMACHeaderMessage {
    pub data_rate: u64,
}
