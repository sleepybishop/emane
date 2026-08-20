#ifndef EMANEMODELSCOMMEFFECTETHERNETPROTOCOLRULE_HEADER_
#define EMANEMODELSCOMMEFFECTETHERNETPROTOCOLRULE_HEADER_

#include "rule.h"
#include "ipprotocolrule.h" 

#include <cstdint>
#include <list>
#include <algorithm>

namespace EMANE
{
  namespace Models
  {
    namespace CommEffect
    {
      class EthernetProtocolRule : public Rule
      {
      public:
        EthernetProtocolRule(){}
        virtual ~EthernetProtocolRule(){}
      };
      
      using EthernetProtocolRules = std::list<EthernetProtocolRule*>;
    }
  }
}

#endif // EMANEMODELSCOMMEFFECTETHERNETPROTOCOLRULE_HEADER_
