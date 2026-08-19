#include "eventagentfactorymanager.h"
#include "rust_ffi.h"

extern "C" {
    void* emane_rs_factory_manager_create_event_agent(const char* lib, uint16_t id, void* platform, char* err_buf, size_t err_buf_len);
}

EMANE::EventAgent * EMANE::EventAgentFactory::createEventAgent(NEMId nemId, PlatformServiceProvider *pPlatformService) const {
    char err_buf[256] = {0};
    void* ptr = emane_rs_factory_manager_create_event_agent(sLibraryName_.c_str(), nemId, pPlatformService, err_buf, sizeof(err_buf));
    if (err_buf[0] != '\0') throw Utils::FactoryException(err_buf);
    return static_cast<EMANE::EventAgent*>(ptr);
}

void EMANE::EventAgentFactory::destoryEventAgent(EMANE::EventAgent * pAgent) const {}

EMANE::EventAgentFactoryManager::EventAgentFactoryManager(){}

EMANE::EventAgentFactoryManager::~EventAgentFactoryManager() {
    for(auto& pair : eventAgentFactoryMap_) delete pair.second;
}

const EMANE::EventAgentFactory & EMANE::EventAgentFactoryManager::getEventAgentFactory(const std::string & sLibraryFile) {
    if(eventAgentFactoryMap_.find(sLibraryFile) == eventAgentFactoryMap_.end()) {
        eventAgentFactoryMap_[sLibraryFile] = new EventAgentFactory(sLibraryFile);
    }
    return *eventAgentFactoryMap_[sLibraryFile];
}
