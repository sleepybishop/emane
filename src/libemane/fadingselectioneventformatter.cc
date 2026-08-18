#include <cstdint>
/*
 * Copyright (c) 2017 - Adjacent Link LLC, Bridgewater, New Jersey
 * All rights reserved.
 */

#include "emane/events/fadingselectioneventformatter.h"

extern "C" {
    void emane_rs_format_fading_selection_element(uint16_t nem_id, int32_t fading_model, void* ctx, void (*add_string)(void*, const char*));
}

EMANE::Events::FadingSelectionEventFormatter::
FadingSelectionEventFormatter(const FadingSelectionEvent & event):
  event_(event)
{}

EMANE::Strings EMANE::Events::FadingSelectionEventFormatter::operator()() const
{
  Strings strings;

  for(const auto & selection : event_.getFadingSelections())
    {
        emane_rs_format_fading_selection_element(
            selection.getNEMId(),
            static_cast<int32_t>(selection.getFadingModel()),
            &strings,
            [](void* ctx, const char* s) {
                static_cast<Strings*>(ctx)->push_back(s);
            }
        );
    }

  return strings;
}
