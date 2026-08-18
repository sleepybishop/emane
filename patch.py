import re
with open('src/libemane/spectrummonitor.cc', 'r') as f:
    text = f.read()

def repl(m):
    return """
  bool emane_rs_filter_match(
        const void* criterion,
        uint64_t freq,
        uint64_t bw,
        uint16_t subid,
        const uint8_t* filter_data_ptr,
        size_t filter_data_len
  ) {
      const EMANE::FilterMatchCriterion* pFilterMatchCriterion = 
          reinterpret_cast<const EMANE::FilterMatchCriterion*>(criterion);
      if(pFilterMatchCriterion) {
          EMANE::FilterElementValues vals;
          vals.u64FrequencyHz_ = freq;
          vals.u64BandwidthHz_ = bw;
          vals.u16SubId_ = subid;
          if (filter_data_ptr && filter_data_len > 0) {
              vals.filterData_ = std::string(reinterpret_cast<const char*>(filter_data_ptr), filter_data_len);
          }
          return (*pFilterMatchCriterion)(vals);
      }
      return false;
  }
"""

text = re.sub(r'  bool emane_rs_filter_match\([\s\S]*?\}', repl, text, count=1)
with open('src/libemane/spectrummonitor.cc', 'w') as f:
    f.write(text)
