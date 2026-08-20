/*
 * Copyright (c) 2015,2023 - Adjacent Link LLC, Bridgewater, New Jersey
 * All rights reserved.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions
 * are met:
 *
 * * Redistributions of source code must retain the above copyright
 *   notice, this list of conditions and the following disclaimer.
 * * Redistributions in binary form must reproduce the above copyright
 *   notice, this list of conditions and the following disclaimer in
 *   the documentation and/or other materials provided with the
 *   distribution.
 * * Neither the name of Adjacent Link LLC nor the names of its
 *   contributors may be used to endorse or promote products derived
 *   from this software without specific prior written permission.
 *
 * THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS
 * "AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT
 * LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS
 * FOR A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE
 * COPYRIGHT OWNER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT,
 * INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING,
 * BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES;
 * LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER
 * CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT
 * LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN
 * ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE
 * POSSIBILITY OF SUCH DAMAGE.
 */

#include "pcrmanager.h"
#include "emane/configurationexception.h"
#include "emane/utils/parameterconvert.h"

#include <cmath>
#include <cstdlib>
#include <limits>


namespace
{
  const char * pzSchema="\
<xs:schema xmlns:xs='http://www.w3.org/2001/XMLSchema'>\
  <xs:simpleType name='SINRType'>\
    <xs:restriction base='xs:token'>\
      <xs:pattern value='[+-]?(0|[1-9][0-9]*)(.[0-9]{1,2})?'/>\
    </xs:restriction>\
  </xs:simpleType>\
  <xs:simpleType name='PORType'>\
    <xs:restriction base='xs:token'>\
      <xs:pattern value='(0|[1-9][0-9]*)(.[0-9]{1,2})?'/>\
    </xs:restriction>\
  </xs:simpleType>\
  <xs:element name='bentpipe-model-pcr'>\
    <xs:complexType>\
      <xs:sequence>\
        <xs:element name='curve' maxOccurs='unbounded'>\
          <xs:complexType>\
            <xs:sequence>\
               <xs:element name='entry' maxOccurs='unbounded'>\
                 <xs:complexType>\
                   <xs:attribute name='sinr' type='SINRType' use='optional'/>\
                   <xs:attribute name='por' type='PORType' use='optional'/>\
                 </xs:complexType>\
               </xs:element>\
            </xs:sequence>\
            <xs:attribute name='index' type='xs:unsignedShort' use='required'/>\
          </xs:complexType>\
        </xs:element>\
      </xs:sequence>\
      <xs:attribute name='packetsize' type='xs:unsignedShort' use='required'/>\
    </xs:complexType>\
  </xs:element>\
</xs:schema>";

  std::string scaleFloatToInteger(const std::string & sValue)
  {
    const int iScaleFactor{2};
    std::string sTmpParameter{sValue};

    // location of decimal point, if exists
    std::string::size_type indexPoint =  sTmpParameter.find(".",0);

    if(indexPoint != std::string::npos)
      {
        std::string::size_type numberOfDigitsAfterPoint =
          sTmpParameter.size() - indexPoint - 1;

        if(numberOfDigitsAfterPoint > iScaleFactor)
          {
            // need to move the decimal point, enough digits are present
            sTmpParameter.insert(sTmpParameter.size() - (numberOfDigitsAfterPoint - iScaleFactor),
                                 ".");
          }
        else
          {
            // need to append 0s
            sTmpParameter.append(iScaleFactor - numberOfDigitsAfterPoint,'0');
          }

        // remove original decimal point
        sTmpParameter.erase(indexPoint,1);
      }
    else
      {
        // need to append 0s
        sTmpParameter.append(iScaleFactor,'0');
      }

    return sTmpParameter;
  }
}

extern "C" {
  void* rust_bentpipe_pcr_manager_new();
  void rust_bentpipe_pcr_manager_free(void* ptr);
  bool rust_bentpipe_pcr_manager_load(void* ptr, const char* filename);
  bool rust_bentpipe_pcr_manager_get_por(void* ptr, uint16_t index, float sinr, size_t packet_length_bytes, float* out_por);
  size_t rust_bentpipe_pcr_manager_get_indices(void* ptr, uint16_t* out_indices, size_t max_indices);
}

EMANE::Models::BentPipe::PCRManager::PCRManager():
  modifierLengthBytes_{0},
  rust_obj_{rust_bentpipe_pcr_manager_new()}
  {}

EMANE::Models::BentPipe::PCRManager::~PCRManager()
{
  if(rust_obj_)
    {
      rust_bentpipe_pcr_manager_free(rust_obj_);
    }
}

void EMANE::Models::BentPipe::PCRManager::load(const std::string & sPCRFileName)
{
  if(!rust_bentpipe_pcr_manager_load(rust_obj_, sPCRFileName.c_str()))
    {
      throw makeException<ConfigurationException>("failed to load or invalid document: %s",
                                                  sPCRFileName.c_str());
    }

  // Populate curveTable_ with empty curves just for getCurveTable() index extraction
  uint16_t indices[1024];
  size_t count = rust_bentpipe_pcr_manager_get_indices(rust_obj_, indices, 1024);
  
  for(size_t i = 0; i < count; ++i)
    {
      Curve emptyCurve;
      curveTable_.insert(std::make_pair(indices[i], std::make_tuple(0, 0, emptyCurve)));
    }
}

std::optional<float> EMANE::Models::BentPipe::PCRManager::getPOR(PCRCurveIndex index,
                                                                 float fSINR,
                                                                 size_t packetLengthBytes) const
{
  float out_por = 0.0f;
  if(rust_bentpipe_pcr_manager_get_por(rust_obj_, index, fSINR, packetLengthBytes, &out_por))
    {
      return out_por;
    }
  return {};
}

const EMANE::Models::BentPipe::PCRManager::CurveTable &
EMANE::Models::BentPipe::PCRManager::PCRManager::getCurveTable() const
{
  return curveTable_;
}
