#include "fadingmanager.h"
#include <iostream>

extern "C" {
  emane_rs_fadingmanager* emane_rs_fadingmanager_new(uint16_t nem_id);
  void emane_rs_fadingmanager_free(emane_rs_fadingmanager* ptr);
  
  void emane_rs_fadingmanager_update_config(emane_rs_fadingmanager* ptr, const char* sType);
  void emane_rs_fadingmanager_update_selection(emane_rs_fadingmanager* ptr, uint16_t nem_id, uint32_t model);

  emane_rs_fadingalgorithmstore* emane_rs_fadingmanager_create_store(emane_rs_fadingmanager* ptr);
}

EMANE::FadingManager::FadingManager(NEMId id,
                                    PlatformServiceProvider * pPlatformService,
                                    const std::string & sPrefix):
  id_{id},
  pPlatformService_{pPlatformService},
  sPrefix_{sPrefix},
  pRustFadingManager_{emane_rs_fadingmanager_new(id)}
{
}

EMANE::FadingManager::~FadingManager()
{
  if(pRustFadingManager_) {
    emane_rs_fadingmanager_free(pRustFadingManager_);
  }
}

void EMANE::FadingManager::initialize(Registrar & registrar)
{
  auto & configRegistrar = registrar.configurationRegistrar();

  std::string sModelDescription{"Defines the fading model:"
                                " none, event, nakagami, lognormal."};

  std::string sModelsRegex{"^(none|event|nakagami|lognormal)$"};

  configRegistrar.registerNonNumeric<std::string>(sPrefix_ + "model",
                                                  EMANE::ConfigurationProperties::DEFAULT |
                                                  EMANE::ConfigurationProperties::MODIFIABLE,
                                                  {"none"},
                                                  sModelDescription,
                                                  1,
                                                  1,
                                                  sModelsRegex);

  // Nakagami parameters
  configRegistrar.registerNumeric<double>(sPrefix_ + "nakagami.distance0",
                                          ConfigurationProperties::DEFAULT | ConfigurationProperties::MODIFIABLE,
                                          {0.0},
                                          "Defines the distance (m) from the source where the nakagami.m0"
                                          " shape factor is applied.");

  configRegistrar.registerNumeric<double>(sPrefix_ + "nakagami.distance1",
                                          ConfigurationProperties::DEFAULT | ConfigurationProperties::MODIFIABLE,
                                          {0.0},
                                          "Defines the distance (m) from the source where the nakagami.m1"
                                          " shape factor is applied.");

  configRegistrar.registerNumeric<double>(sPrefix_ + "nakagami.distance2",
                                          ConfigurationProperties::DEFAULT | ConfigurationProperties::MODIFIABLE,
                                          {0.0},
                                          "Defines the distance (m) from the source where the nakagami.m2"
                                          " shape factor is applied.");

  configRegistrar.registerNumeric<double>(sPrefix_ + "nakagami.m0",
                                          ConfigurationProperties::DEFAULT | ConfigurationProperties::MODIFIABLE,
                                          {0.75},
                                          "Defines the nakagami-m shape factor (0.5 to less than 1.0) applied"
                                          " to distances > nakagami.distance0 and <= nakagami.distance1.");

  configRegistrar.registerNumeric<double>(sPrefix_ + "nakagami.m1",
                                          ConfigurationProperties::DEFAULT | ConfigurationProperties::MODIFIABLE,
                                          {1.0},
                                          "Defines the nakagami-m shape factor (1.0) applied to distances > "
                                          " nakagami.distance1 and <= nakagami.distance2. 1.0 models rayleigh fading.");

  configRegistrar.registerNumeric<double>(sPrefix_ + "nakagami.m2",
                                          ConfigurationProperties::DEFAULT | ConfigurationProperties::MODIFIABLE,
                                          {200.0},
                                          "Defines the nakagami-m shape factor (> 1.0) applied to distances"
                                          " > nakagami.distance2.");
}

void EMANE::FadingManager::configure(const ConfigurationUpdate & update)
{
  configure_i(update);
}

void EMANE::FadingManager::modify(const ConfigurationUpdate & update)
{
  configure_i(update);
}

void EMANE::FadingManager::configure_i(const ConfigurationUpdate & update)
{
  for(const auto & item : update)
    {
      if(item.first == sPrefix_ + "model")
        {
          std::string sType{item.second[0].asString()};
          emane_rs_fadingmanager_update_config(pRustFadingManager_, sType.c_str());
        }
    }
}

void EMANE::FadingManager::update(const Events::FadingSelections & fadingSelections)
{
  for(const auto & selection : fadingSelections)
    {
      emane_rs_fadingmanager_update_selection(pRustFadingManager_, selection.getNEMId(), static_cast<uint32_t>(selection.getFadingModel()));
    }
}

EMANE::FadingAlgorithmStore
EMANE::FadingManager::createFadingAlgorithmStore() const
{
  return emane_rs_fadingmanager_create_store(pRustFadingManager_);
}

std::pair<EMANE::FadingInfo,bool>
EMANE::FadingManager::getFadingSelection(NEMId nemId) const
{
  // Rust fading manager handles the selection!
  // Wait, ReceiveProcessor needs this? We will port ReceiveProcessor anyway!
  return {{Events::FadingModel::NONE,nullptr},true};
}
