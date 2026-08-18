import re

with open("src/libemane/receiveprocessor.cc", "r") as f:
    content = f.read()

# Replace the broken list access
bad_code = """        auto tx = static_cast<const EMANE::CommonPHYHeader*>(hdr_ptr)->getTransmitters()[idx];"""
good_code = """        auto txs = static_cast<const EMANE::CommonPHYHeader*>(hdr_ptr)->getTransmitters();
        auto it = txs.begin();
        std::advance(it, idx);
        auto tx = *it;"""

content = content.replace(bad_code, good_code)

with open("src/libemane/receiveprocessor.cc", "w") as f:
    f.write(content)
