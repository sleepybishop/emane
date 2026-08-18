with open("src/libemane/nopfiledescriptorservice.cc", "w") as f:
    f.write("""#include <cstdint>
/*
 * Copyright (c) 2016 - Adjacent Link LLC, Bridgewater, New
 * Jersey
 * All rights reserved.
 */

#include "nopfiledescriptorservice.h"

extern "C" {
    void emane_rs_nop_file_descriptor_service();
}

EMANE::NOPFileDescriptorService::~NOPFileDescriptorService(){};

void EMANE::NOPFileDescriptorService::removeFileDescriptor(int)
{
  emane_rs_nop_file_descriptor_service();
}

void EMANE::NOPFileDescriptorService::addFileDescriptor_i(int,
                                                          DescriptorType,
                                                          Callback)
{
  emane_rs_nop_file_descriptor_service();
}
""")
