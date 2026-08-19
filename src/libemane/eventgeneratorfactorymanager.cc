#include "eventgeneratorfactorymanager.h"
#include "rust_ffi.h"
#include "emane/utils/factoryexception.h"

extern "C" {
    void* emane_rs_factory_manager_create_event_generator(const char* lib, void* platform, char* err_buf, size_t err_buf_len);
}

EMANE::EventGenerator * EMANE::EventGeneratorFactory::createEventGenerator(PlatformServiceProvider *pPlatformService) const {
    char err_buf[256] = {0};
    void* ptr = emane_rs_factory_manager_create_event_generator(sLibraryName_.c_str(), pPlatformService, err_buf, sizeof(err_buf));
    if (err_buf[0] != '\0') throw Utils::FactoryException(err_buf);
    return static_cast<EMANE::EventGenerator*>(ptr);
}

void EMANE::EventGeneratorFactory::destoryEventGenerator(EMANE::EventGenerator * pGenerator) const {}

EMANE::EventGeneratorFactoryManager::EventGeneratorFactoryManager(){}

EMANE::EventGeneratorFactoryManager::~EventGeneratorFactoryManager() {
    for(auto& pair : eventGeneratorFactoryMap_) delete pair.second;
}

const EMANE::EventGeneratorFactory & EMANE::EventGeneratorFactoryManager::getEventGeneratorFactory(const std::string & sLibraryFile) {
    if(eventGeneratorFactoryMap_.find(sLibraryFile) == eventGeneratorFactoryMap_.end()) {
        eventGeneratorFactoryMap_[sLibraryFile] = new EventGeneratorFactory(sLibraryFile);
    }
    return *eventGeneratorFactoryMap_[sLibraryFile];
}
