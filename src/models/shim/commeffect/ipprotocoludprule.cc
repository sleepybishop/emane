#include "ipprotocoludprule.h"

extern "C" void* ip_protocol_udp_rule_new(uint16_t src, uint16_t dst);
extern "C" void ip_protocol_udp_rule_drop(void* ptr);

EMANE::Models::CommEffect::IPProtocolUDPRule::IPProtocolUDPRule(std::uint16_t u16SrcPort, std::uint16_t u16DstPort) {
    pRustObj = ip_protocol_udp_rule_new(u16SrcPort, u16DstPort);
}

EMANE::Models::CommEffect::IPProtocolUDPRule::~IPProtocolUDPRule() {
    ip_protocol_udp_rule_drop(pRustObj);
}

bool EMANE::Models::CommEffect::IPProtocolUDPRule::match(const void * buf, std::size_t len, std::uint16_t u16Type) {
    return false;
}
