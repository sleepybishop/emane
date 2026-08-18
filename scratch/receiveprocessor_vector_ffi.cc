#include "receiveprocessor.h"
#include "emane/locationinfo.h"

extern "C" {
    size_t emane_c_location_infos_size(const void* vec_ptr) {
        auto vec = static_cast<const std::vector<std::pair<EMANE::LocationInfo, bool>>*>(vec_ptr);
        return vec->size();
    }
    
    const void* emane_c_location_infos_get(const void* vec_ptr, size_t index, bool* out_bool) {
        auto vec = static_cast<const std::vector<std::pair<EMANE::LocationInfo, bool>>*>(vec_ptr);
        if(out_bool) *out_bool = (*vec)[index].second;
        return &(*vec)[index].first;
    }

    size_t emane_c_fading_infos_size(const void* vec_ptr) {
        auto vec = static_cast<const std::vector<std::pair<EMANE::FadingInfo, bool>>*>(vec_ptr);
        return vec->size();
    }
    
    const void* emane_c_fading_infos_get(const void* vec_ptr, size_t index, bool* out_bool, uint32_t* out_model) {
        auto vec = static_cast<const std::vector<std::pair<EMANE::FadingInfo, bool>>*>(vec_ptr);
        if(out_bool) *out_bool = (*vec)[index].second;
        if(out_model) *out_model = static_cast<uint32_t>((*vec)[index].first.first);
        return (*vec)[index].first.second;
    }
}
