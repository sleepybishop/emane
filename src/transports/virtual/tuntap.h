#ifndef TUNTAP_HEADER_
#define TUNTAP_HEADER_

#include "emane/inetaddr.h"
#include <string>
#include "emane/platformserviceprovider.h"
#include "emane/utils/netutils.h"

namespace EMANE
{
  namespace Transports
  {
    namespace Virtual
    {
      class TunTap
      {
      private:
        PlatformServiceProvider * pPlatformService_;
        void* rust_obj_;
        INETAddr tunAddr_;
        INETAddr tunMask_;

      public:
        TunTap(PlatformServiceProvider * pPlatformService);
        ~TunTap();

        int open(const char *, const char *);
        int close();
        int activate(bool);
        int deactivate();
        int get_handle();
        int readv(struct iovec*, size_t);
        int writev(const struct iovec*, size_t);
        int set_addr(const INETAddr &, const INETAddr &);
        int set_ethaddr(const Utils::EtherAddr &);
        int set_ethaddr(NEMId);
        INETAddr & get_addr();
        INETAddr & get_mask();
      };
    }
  }
}
#endif // TUNTAP_HEADER_
