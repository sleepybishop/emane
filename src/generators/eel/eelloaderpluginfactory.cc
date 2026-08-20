#include "eelloaderpluginfactory.h"
#include <string>

extern "C" {
    void* emane_rs_eel_loader_plugin_factory_construct(const char* library_name, char** error_out);
    void emane_rs_eel_loader_plugin_factory_free(void* ptr);
    void* emane_rs_eel_loader_plugin_factory_create_plugin(const void* ptr);
    void emane_rs_eel_loader_plugin_factory_destroy_plugin(const void* ptr, void* plugin);
    void emane_rs_eel_loader_plugin_factory_free_error(char* err);
}

EMANE::Generators::EEL::LoaderPluginFactory::LoaderPluginFactory():
  pLibHandle_{nullptr},
  createPluginFunc_{nullptr},
  destroyPluginFunc_{nullptr}{}

void EMANE::Generators::EEL::LoaderPluginFactory::construct(const std::string & sLibraryName)
{
    char* error_out = nullptr;
    void* ptr = emane_rs_eel_loader_plugin_factory_construct(sLibraryName.c_str(), &error_out);
    if(error_out) {
        std::string err(error_out);
        emane_rs_eel_loader_plugin_factory_free_error(error_out);
        throw Utils::FactoryException(err);
    }
    pLibHandle_ = ptr;
}

EMANE::Generators::EEL::LoaderPluginFactory::~LoaderPluginFactory()
{
    if(pLibHandle_) {
        emane_rs_eel_loader_plugin_factory_free(pLibHandle_);
    }
}

EMANE::Generators::EEL::LoaderPlugin * EMANE::Generators::EEL::LoaderPluginFactory::createPlugin() const
{
    if(pLibHandle_) {
        return static_cast<EMANE::Generators::EEL::LoaderPlugin*>(emane_rs_eel_loader_plugin_factory_create_plugin(pLibHandle_));
    }
    return nullptr;
}

void EMANE::Generators::EEL::LoaderPluginFactory::destoryPlugin(LoaderPlugin * pPlugin) const
{
    if(pLibHandle_) {
        emane_rs_eel_loader_plugin_factory_destroy_plugin(pLibHandle_, pPlugin);
    }
}
