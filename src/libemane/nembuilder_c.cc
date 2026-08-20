#include <cstdint>
#include "emane/configurationupdate.h"
#include "emane/application/nembuilder.h"
#include <string>

extern "C" {

struct FfiStringArray {
    const char** data;
    size_t len;
};

struct FfiConfigUpdateReqItem {
    const char* name;
    FfiStringArray values;
};

struct FfiConfigUpdateReq {
    const FfiConfigUpdateReqItem* data;
    size_t len;
};

EMANE::ConfigurationUpdateRequest convertReq(const FfiConfigUpdateReq& req) {
    EMANE::ConfigurationUpdateRequest cppReq;
    for(size_t i = 0; i < req.len; ++i) {
        const auto& item = req.data[i];
        std::vector<std::string> vals;
        for(size_t j = 0; j < item.values.len; ++j) {
            vals.push_back(item.values.data[j]);
        }
        cppReq.push_back(std::make_pair(item.name, vals));
    }
    return cppReq;
}

void* emane_rs_ffi_build_phy_layer(uint16_t id, const char* sLibraryFile, FfiConfigUpdateReq req, bool bSkipConfigure) {
    std::string libFile = sLibraryFile ? sLibraryFile : "";
    EMANE::Application::NEMBuilder builder;
    auto layer = builder.buildPHYLayer(id, libFile, convertReq(req), bSkipConfigure);
    return layer.release();
}

void* emane_rs_ffi_build_mac_layer(uint16_t id, const char* sLibraryFile, FfiConfigUpdateReq req, bool bSkipConfigure) {
    std::string libFile = sLibraryFile ? sLibraryFile : "";
    EMANE::Application::NEMBuilder builder;
    auto layer = builder.buildMACLayer(id, libFile, convertReq(req), bSkipConfigure);
    return layer.release();
}

void* emane_rs_ffi_build_transport_layer(uint16_t id, const char* sLibraryFile, FfiConfigUpdateReq req, bool bSkipConfigure) {
    std::string libFile = sLibraryFile ? sLibraryFile : "";
    EMANE::Application::NEMBuilder builder;
    auto layer = builder.buildTransportLayer(id, libFile, convertReq(req), bSkipConfigure);
    return layer.release();
}

void* emane_rs_ffi_build_shim_layer(uint16_t id, const char* sLibraryFile, FfiConfigUpdateReq req, bool bSkipConfigure) {
    std::string libFile = sLibraryFile ? sLibraryFile : "";
    EMANE::Application::NEMBuilder builder;
    auto layer = builder.buildShimLayer(id, libFile, convertReq(req), bSkipConfigure);
    return layer.release();
}

void* emane_rs_ffi_build_nem(uint16_t id, void** layers, size_t num_layers, FfiConfigUpdateReq req, bool bExternalTransport) {
    EMANE::Application::NEMBuilder builder;
    EMANE::Application::NEMLayers cppLayers;
    for(size_t i = 0; i < num_layers; ++i) {
        cppLayers.push_back(std::unique_ptr<EMANE::NEMLayer>(static_cast<EMANE::NEMLayer*>(layers[i])));
    }
    auto nem = builder.buildNEM(id, cppLayers, convertReq(req), bExternalTransport);
    return nem.release();
}

}

extern "C" uint16_t emane_rs_ffi_component_get_build_id(void* component) {
    if (!component) return 0;
    return static_cast<EMANE::Buildable*>(component)->getBuildId();
}
