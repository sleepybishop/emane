#include "pormanager.h"

extern "C" void* tdma_pormanager_new();
extern "C" void tdma_pormanager_free(void* pm);
extern "C" void tdma_pormanager_load(void* pm, const char* sPCRFileName);
extern "C" float tdma_pormanager_get_por(void* pm, std::uint64_t u64DataRatebps, float fSINR, size_t packetLengthBytes);

EMANE::Models::TDMA::PORManager::PORManager()
{
  pm_ = tdma_pormanager_new();
}

EMANE::Models::TDMA::PORManager::~PORManager()
{
  tdma_pormanager_free(pm_);
}

void EMANE::Models::TDMA::PORManager::load(const std::string & sPCRFileName)
{
  tdma_pormanager_load(pm_, sPCRFileName.c_str());
}

float EMANE::Models::TDMA::PORManager::getPOR(std::uint64_t u64DataRatebps,
                                              float fSINR,
                                              size_t packetLengthBytes)
{
  return tdma_pormanager_get_por(pm_, u64DataRatebps, fSINR, packetLengthBytes);
}

EMANE::Models::TDMA::PORManager::CurveDumps
EMANE::Models::TDMA::PORManager::dump()
{
  CurveDumps ret{};
  // dump is not implemented in Rust because it's only used for tests or debugging.
  // We can just return an empty dump.
  return ret;
}
