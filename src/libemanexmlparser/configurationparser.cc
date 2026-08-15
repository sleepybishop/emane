/*
 * Copyright (c) 2013 - Adjacent Link LLC, Bridgewater, New Jersey
 * Copyright (c) 2009-2010 - DRS CenGen, LLC, Columbia, Maryland
 * All rights reserved.
 */

#include "configurationparser.h"
#include "emanexml.h"

EMANE::ConfigurationParser::ConfigurationParser()
{
}


EMANE::ConfigurationParser::~ConfigurationParser()
{
}


xmlDocPtr 
EMANE::ConfigurationParser::parse(const std::string &sURI)
{
  xmlDocPtr pDoc = emane_rs_xml_parse(sURI.c_str());

  if (!pDoc)
    {
      throw makeException<ParseException>(
                         "Failed to parse document in %s.\n\n"
                         "Possible reason(s):\n"
                         " * Document '%s' does not exist or is invalid XML.\n", 
                         sURI.c_str(), 
                         sURI.c_str());
    }

  return pDoc;
}
