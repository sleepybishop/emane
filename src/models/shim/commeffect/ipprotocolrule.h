#ifndef EMANEMODELSCOMMEFFECTIPPROTOCOLRULE_HEADER_
#define EMANEMODELSCOMMEFFECTIPPROTOCOLRULE_HEADER_

#include "rule.h"

#include <cstdint>
#include <list>

namespace EMANE
{
  namespace Models
  {
    namespace CommEffect
    {
      class IPProtocolRule : public Rule
      {
      public:
        virtual ~IPProtocolRule(){}
        virtual void* getRustObj() const = 0;
        virtual bool isUdp() const = 0;
      };

      using IPProtocolRules = std::list<IPProtocolRule *>;
    }
  }
}

#endif // EMANEMODELSCOMMEFFECTIPPROTOCOLRULE_HEADER_
