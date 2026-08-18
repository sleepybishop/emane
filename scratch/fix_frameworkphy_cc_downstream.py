import sys

with open("src/libemane/frameworkphy.cc", "r") as f:
    lines = f.readlines()

# Add extern "C" declaration
for i, line in enumerate(lines):
    if "void emane_rs_framework_phy_process_upstream_packet(void* ptr, const void* hdr, void* pkt, const void* msgs);" in line:
        lines.insert(i + 1, "    void emane_rs_framework_phy_process_downstream_packet(void* ptr, void* pkt, const void* msgs);\n")
        break

start_idx = -1
end_idx = -1
for i, line in enumerate(lines):
    if "void EMANE::FrameworkPHY::processDownstreamPacket(DownstreamPacket & pkt," in line:
        start_idx = i
    if "void EMANE::FrameworkPHY::processUpstreamPacket(const CommonPHYHeader & commonPHYHeader," in line:
        end_idx = i - 1
        break

downstream = """void EMANE::FrameworkPHY::processDownstreamPacket(DownstreamPacket & pkt,
                                                  const ControlMessages & msgs)
{
  emane_rs_framework_phy_process_downstream_packet(rs_state_, &pkt, &msgs);
}

void EMANE::FrameworkPHY::processDownstreamPacket_i(const TimePoint &,
                                                    DownstreamPacket &,
                                                    const ControlMessages &)
{
}

"""
if start_idx != -1 and end_idx != -1:
    lines = lines[:start_idx] + [downstream] + lines[end_idx:]

with open("src/libemane/frameworkphy.cc", "w") as f:
    f.writelines(lines)
