#ifndef EMANEAPPLICATIONNEMIMPL_HEADER_
#define EMANEAPPLICATIONNEMIMPL_HEADER_

#include "emane/application/nem.h"

#include "nemlayerstack.h"
#include "nemotaadapter.h"
#include "nemnetworkadapter.h"

#include "emane/component.h"

#include <memory>

namespace EMANE
{
  namespace Application
  {
    class NEMImpl : public NEM
    {
    public:
      NEMImpl(NEMId id,
              std::unique_ptr<NEMLayerStack> & pNEMLayerStack,
              bool bExternalTransport);

      ~NEMImpl();

      void initialize(Registrar & registrar) override;

      void configure(const ConfigurationUpdate & update) override;

      void start() override;

      void postStart() override;

      void stop() override;

      void destroy() throw() override;

      NEMId getNEMId() const override;

    private:
      std::unique_ptr<NEMLayerStack> pNEMLayerStack_;
      NEMId id_;
      bool bExternalTransport_;

      std::unique_ptr<NEMOTAAdapter> pNEMOTAAdapter_;
      std::unique_ptr<NEMNetworkAdapter> pNEMNetworkAdapter_;
      
      void* pRsNemImpl_ = nullptr;

      INETAddr platformEndpointAddr_;
      INETAddr transportEndpointAddr_;
      NEMNetworkAdapter::Protocol protocol_;

      // prevent NEM copies
      NEMImpl(const NEMImpl &) = delete;
      NEMImpl & operator=(const NEMImpl &) = delete;
    };
  }
}

#endif // EMANEAPPLICATIONNEMIMPL_HEADER_
