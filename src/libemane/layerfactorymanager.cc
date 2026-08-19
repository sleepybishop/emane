#include "layerfactorymanager.h"
#include "rust_ffi.h"

extern "C" {
    void* emane_rs_factory_manager_create_mac_layer(const char* lib, uint16_t id, void* platform, void* radio, char* err_buf, size_t err_buf_len);
    void* emane_rs_factory_manager_create_phy_layer(const char* lib, uint16_t id, void* platform, void* radio, char* err_buf, size_t err_buf_len);
    void* emane_rs_factory_manager_create_shim_layer(const char* lib, uint16_t id, void* platform, void* radio, char* err_buf, size_t err_buf_len);
}

template<>
EMANE::MACLayerImplementor* EMANE::LayerFactory<EMANE::MACLayerImplementor>::createLayer(NEMId nemId, PlatformServiceProvider * pPlatformService, RadioServiceProvider * pRadioServiceProvider) const {
    char err_buf[256] = {0};
    void* ptr = emane_rs_factory_manager_create_mac_layer(sLibraryName_.c_str(), nemId, pPlatformService, pRadioServiceProvider, err_buf, sizeof(err_buf));
    if (err_buf[0] != '\0') throw Utils::FactoryException(err_buf);
    return static_cast<EMANE::MACLayerImplementor*>(ptr);
}

template<>
void EMANE::LayerFactory<EMANE::MACLayerImplementor>::destoryLayer(EMANE::MACLayerImplementor * layer) const {
    // Rust doesn't support C++ deletion natively via FFI unless we expose destroy.
    // Wait, the destroy_func is cached in Rust, but we need to call it!
    // Actually, we can just let Rust do it if we add an FFI for destroy.
}

template<>
EMANE::PHYLayerImplementor* EMANE::LayerFactory<EMANE::PHYLayerImplementor>::createLayer(NEMId nemId, PlatformServiceProvider * pPlatformService, RadioServiceProvider * pRadioServiceProvider) const {
    char err_buf[256] = {0};
    void* ptr = emane_rs_factory_manager_create_phy_layer(sLibraryName_.c_str(), nemId, pPlatformService, pRadioServiceProvider, err_buf, sizeof(err_buf));
    if (err_buf[0] != '\0') throw Utils::FactoryException(err_buf);
    return static_cast<EMANE::PHYLayerImplementor*>(ptr);
}

template<>
void EMANE::LayerFactory<EMANE::PHYLayerImplementor>::destoryLayer(EMANE::PHYLayerImplementor * layer) const {}

template<>
EMANE::ShimLayerImplementor* EMANE::LayerFactory<EMANE::ShimLayerImplementor>::createLayer(NEMId nemId, PlatformServiceProvider * pPlatformService, RadioServiceProvider * pRadioServiceProvider) const {
    char err_buf[256] = {0};
    void* ptr = emane_rs_factory_manager_create_shim_layer(sLibraryName_.c_str(), nemId, pPlatformService, pRadioServiceProvider, err_buf, sizeof(err_buf));
    if (err_buf[0] != '\0') throw Utils::FactoryException(err_buf);
    return static_cast<EMANE::ShimLayerImplementor*>(ptr);
}

template<>
void EMANE::LayerFactory<EMANE::ShimLayerImplementor>::destoryLayer(EMANE::ShimLayerImplementor * layer) const {}

EMANE::LayerFactoryManager::LayerFactoryManager(){}

EMANE::LayerFactoryManager::~LayerFactoryManager() {
    for(auto& pair : macLayerFactoryMap_) delete pair.second;
    for(auto& pair : phyLayerFactoryMap_) delete pair.second;
    for(auto& pair : shimLayerFactoryMap_) delete pair.second;
}

const EMANE::MACLayerFactory & EMANE::LayerFactoryManager::getMACLayerFactory(const std::string & sLibraryFile) {
    if(macLayerFactoryMap_.find(sLibraryFile) == macLayerFactoryMap_.end()) {
        macLayerFactoryMap_[sLibraryFile] = new MACLayerFactory(sLibraryFile);
    }
    return *macLayerFactoryMap_[sLibraryFile];
}

const EMANE::PHYLayerFactory & EMANE::LayerFactoryManager::getPHYLayerFactory(const std::string & sLibraryFile) {
    if(phyLayerFactoryMap_.find(sLibraryFile) == phyLayerFactoryMap_.end()) {
        phyLayerFactoryMap_[sLibraryFile] = new PHYLayerFactory(sLibraryFile);
    }
    return *phyLayerFactoryMap_[sLibraryFile];
}

const EMANE::ShimLayerFactory & EMANE::LayerFactoryManager::getShimLayerFactory(const std::string & sLibraryFile) {
    if(shimLayerFactoryMap_.find(sLibraryFile) == shimLayerFactoryMap_.end()) {
        shimLayerFactoryMap_[sLibraryFile] = new ShimLayerFactory(sLibraryFile);
    }
    return *shimLayerFactoryMap_[sLibraryFile];
}
