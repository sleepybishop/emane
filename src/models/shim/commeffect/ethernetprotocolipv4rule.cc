#include "ethernetprotocolipv4rule.h"

extern "C" void* ethernet_protocol_ipv4_rule_new(uint32_t src, uint32_t dst, uint16_t len, uint8_t tos, uint8_t ttl);
extern "C" void ethernet_protocol_ipv4_rule_add_ip_rule(void* eth_ptr, void* ip_rule_ptr, bool is_udp);
extern "C" void ethernet_protocol_ipv4_rule_drop(void* ptr);
extern "C" bool ethernet_protocol_ipv4_rule_match(const void* ptr, const void* buf, size_t len, uint16_t type);

EMANE::Models::CommEffect::EthernetProtocolIPv4Rule::EthernetProtocolIPv4Rule(
    std::uint32_t u32Src, std::uint32_t u32Dst, std::uint16_t u16Len,
    std::uint8_t u8TOS, std::uint8_t u8TTL, const IPProtocolRules & rules) 
{ 
    pRustObj = ethernet_protocol_ipv4_rule_new(u32Src, u32Dst, u16Len, u8TOS, u8TTL);
    
    // Transfer ownership of IP rules into the Rust object
    for(auto rule : rules) {
        ethernet_protocol_ipv4_rule_add_ip_rule(pRustObj, rule->getRustObj(), rule->isUdp());
    }
}

EMANE::Models::CommEffect::EthernetProtocolIPv4Rule::~EthernetProtocolIPv4Rule() {
    ethernet_protocol_ipv4_rule_drop(pRustObj);
}

bool EMANE::Models::CommEffect::EthernetProtocolIPv4Rule::match(const void * buf, std::size_t len, std::uint16_t u16Type) {
    return ethernet_protocol_ipv4_rule_match(pRustObj, buf, len, u16Type);
}
