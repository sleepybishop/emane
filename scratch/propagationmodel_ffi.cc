#include "propagationmodelalgorithm.h"
#include <vector>

extern "C" {
    struct EMANE_PathlossResult {
        double* pathlosses;
        size_t count;
        bool success;
    };

    EMANE_PathlossResult emane_c_propagation_model_compute(void* algo_ptr,
                                                           uint16_t src,
                                                           const void* loc_info_ptr,
                                                           const void* segments_ptr) {
        auto algo = static_cast<EMANE::PropagationModelAlgorithm*>(algo_ptr);
        auto loc = static_cast<const EMANE::LocationInfo*>(loc_info_ptr);
        auto segs = static_cast<const EMANE::FrequencySegments*>(segments_ptr);
        
        auto res = (*algo)(src, *loc, *segs);
        
        double* arr = nullptr;
        if (res.second && !res.first.empty()) {
            arr = new double[res.first.size()];
            for (size_t i = 0; i < res.first.size(); ++i) {
                arr[i] = res.first[i];
            }
        }
        
        return {
            arr,
            res.first.size(),
            res.second
        };
    }
    
    void emane_c_propagation_model_free_result(EMANE_PathlossResult* res) {
        if (res && res->pathlosses) {
            delete[] res->pathlosses;
        }
    }
}
