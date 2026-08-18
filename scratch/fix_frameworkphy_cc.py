import sys

with open("src/libemane/frameworkphy.cc", "r") as f:
    lines = f.readlines()

headers = """#include "frameworkphy.h"
#include "rust_ffi.h"

extern "C" {
    void* emane_rs_framework_phy_create(void* cpp_this);
    void emane_rs_framework_phy_destroy(void* ptr);
    void emane_rs_framework_phy_process_upstream_packet(void* ptr, const void* hdr, void* pkt, const void* msgs);
}
"""
for i, line in enumerate(lines):
    if '#include "frameworkphy.h"' in line:
        lines[i] = headers
        break

for i, line in enumerate(lines):
    if "PHYLayerImplementor{id, pPlatformService}," in line:
        lines[i] = "  PHYLayerImplementor{id, pPlatformService},\n  rs_state_{emane_rs_framework_phy_create(this)},\n"
        break

for i, line in enumerate(lines):
    if "EMANE::FrameworkPHY::~FrameworkPHY(){}" in line:
        lines[i] = "EMANE::FrameworkPHY::~FrameworkPHY() { emane_rs_framework_phy_destroy(rs_state_); }\n"
        break

start_idx = -1
end_idx = -1
for i, line in enumerate(lines):
    if "void EMANE::FrameworkPHY::processUpstreamPacket(const CommonPHYHeader & commonPHYHeader," in line:
        start_idx = i
    if "void EMANE::FrameworkPHY::processEvent(" in line:
        end_idx = i - 1
        break

upstream = """void EMANE::FrameworkPHY::processUpstreamPacket(const CommonPHYHeader & commonPHYHeader,
                                                UpstreamPacket & pkt,
                                                const ControlMessages & msgs)
{
  emane_rs_framework_phy_process_upstream_packet(rs_state_, &commonPHYHeader, &pkt, &msgs);
}

void EMANE::FrameworkPHY::processUpstreamPacket_i(const TimePoint &,
                                                  const CommonPHYHeader &,
                                                  UpstreamPacket &,
                                                  const ControlMessages &)
{
}

"""
if start_idx != -1 and end_idx != -1:
    lines = lines[:start_idx] + [upstream] + lines[end_idx:]

with open("src/libemane/frameworkphy.cc", "w") as f:
    f.writelines(lines)
