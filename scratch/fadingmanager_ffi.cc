#include "fadingmanager.h"
#include <string>

extern "C" {
    double emane_c_fading_store_compute(void* store_ptr, const char* name, double rx_power) {
        auto store = static_cast<EMANE::FadingAlgorithmStore*>(store_ptr);
        auto iter = store->find(name);
        if(iter != store->end()) {
            return (*iter->second)(rx_power);
        }
        return rx_power;
    }

    double emane_c_fading_store_compute_sinr(void* store_ptr, const char* name, double sinr, double rx_power) {
        auto store = static_cast<EMANE::FadingAlgorithmStore*>(store_ptr);
        auto iter = store->find(name);
        if(iter != store->end()) {
            return (*iter->second)(sinr, rx_power);
        }
        return rx_power;
    }
}
