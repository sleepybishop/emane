#ifndef EMANEMODELSTDMAPORMANAGER_HEADER_
#define EMANEMODELSTDMAPORMANAGER_HEADER_

#include <string>
#include <map>
#include <cstdint>

namespace EMANE
{
  namespace Models
  {
    namespace TDMA
    {
      class PORManager
      {
      public:
        PORManager();
        ~PORManager();

        void load(const std::string & sPCRFileName);

        float getPOR(std::uint64_t u64DataRate,
                     float fSINR,
                     size_t packetLengthBytes);

        using CurveDump = std::map<float,float>;
        using CurveDumps = std::map<std::uint64_t,CurveDump>;
        CurveDumps dump();

      private:
        void* pm_;
        
        PORManager(const PORManager &) = delete;
        PORManager & operator=(const PORManager &) = delete;
      };
    }
  }
}
#endif // EMANEMODELSTDMAPORMANAGER_HEADER_
