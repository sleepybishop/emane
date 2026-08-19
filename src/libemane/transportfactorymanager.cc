#include "transportfactorymanager.h"
#include "rust_ffi.h"

extern "C" {
    void* emane_rs_factory_manager_create_transport(const char* lib, uint16_t id, void* platform, char* err_buf, size_t err_buf_len);
}

EMANE::Transport * EMANE::TransportFactory::createTransport(NEMId nemId, PlatformServiceProvider *pPlatformService) const {
    char err_buf[256] = {0};
    void* ptr = emane_rs_factory_manager_create_transport(sLibraryName_.c_str(), nemId, pPlatformService, err_buf, sizeof(err_buf));
    if (err_buf[0] != '\0') throw Utils::FactoryException(err_buf);
    return static_cast<EMANE::Transport*>(ptr);
}

void EMANE::TransportFactory::destoryTransport(EMANE::Transport * pTransport) const {}

EMANE::TransportFactoryManager::TransportFactoryManager(){}

EMANE::TransportFactoryManager::~TransportFactoryManager() {
    for(auto& pair : transportFactoryMap_) delete pair.second;
}

const EMANE::TransportFactory & EMANE::TransportFactoryManager::getTransportFactory(const std::string & sLibraryFile) {
    if(transportFactoryMap_.find(sLibraryFile) == transportFactoryMap_.end()) {
        transportFactoryMap_[sLibraryFile] = new TransportFactory(sLibraryFile);
    }
    return *transportFactoryMap_[sLibraryFile];
}
