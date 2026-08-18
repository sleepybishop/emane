with open("src/libemane/socket.cc", "w") as f:
    f.write("""#include <cstdint>
/*
 * Copyright (c) 2015 - Adjacent Link LLC, Bridgewater, New Jersey
 * All rights reserved.
 */

#include "socket.h"
#include "socketexception.h"
#include <sys/socket.h>
#include <sys/types.h>

extern "C" {
    int emane_rs_socket_close(int sock);
    ssize_t emane_rs_socket_sendto(int sock, const void * buf, size_t len, int flags, const struct sockaddr * addr, socklen_t addrlen);
}

void EMANE::Socket::close()
{
  iSock_ = emane_rs_socket_close(iSock_);
}

ssize_t EMANE::Socket::sendto(const void * buf,size_t len,int flags,const INETAddr & address)
{
  return emane_rs_socket_sendto(iSock_, buf, len, flags, address.getSockAddr(), address.getAddrLength());
}


EMANE::Socket::Socket():
  iSock_{-1}{}

EMANE::Socket::~Socket()
{
  close();
}
""")
