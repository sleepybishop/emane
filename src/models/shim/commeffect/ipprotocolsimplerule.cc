#include "ipprotocolsimplerule.h"

extern "C" void* ip_protocol_simple_rule_new(uint8_t u8Type);
extern "C" void ip_protocol_simple_rule_drop(void* ptr);

EMANE::Models::CommEffect::IPProtocolSimpleRule::IPProtocolSimpleRule(std::uint8_t u8Type) {
    pRustObj = ip_protocol_simple_rule_new(u8Type);
}

EMANE::Models::CommEffect::IPProtocolSimpleRule::~IPProtocolSimpleRule() {
    ip_protocol_simple_rule_drop(pRustObj);
}

bool EMANE::Models::CommEffect::IPProtocolSimpleRule::match(const void * buf, std::size_t len, std::uint16_t u16Type) {
    return false;
}
