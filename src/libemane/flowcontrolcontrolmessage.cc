#include <cstdint>
/*
 * Copyright (c) 2013-2014,2016 - Adjacent Link LLC, Bridgewater,
 * New Jersey
 * All rights reserved.
 */

#include "emane/controls/flowcontrolcontrolmessage.h"
#include "flowcontrol.pb.h"

extern "C" {
    void* emane_rs_controls_flow_control_create(uint16_t tokens);
    void* emane_rs_controls_flow_control_clone(const void* ptr);
    void emane_rs_controls_flow_control_destroy(void* ptr);
}

class EMANE::Controls::FlowControlControlMessage::Implementation
{
public:
  Implementation(std::uint16_t u16Tokens):
    u16Tokens_{u16Tokens}
  {
      pRsMsg_ = emane_rs_controls_flow_control_create(u16Tokens_);
  }

  Implementation(const Implementation& other) :
    u16Tokens_{other.u16Tokens_}
  {
      pRsMsg_ = emane_rs_controls_flow_control_clone(other.pRsMsg_);
  }

  ~Implementation() {
      emane_rs_controls_flow_control_destroy(pRsMsg_);
  }

  std::uint16_t getTokens() const
  {
    return u16Tokens_;
  }

  Implementation* clone() const {
      return new Implementation(*this);
  }

private:
  void* pRsMsg_;
  const std::uint16_t u16Tokens_;
};

EMANE::Controls::FlowControlControlMessage::
FlowControlControlMessage(const FlowControlControlMessage & msg):
  ControlMessage{IDENTIFIER},
  pImpl_{msg.pImpl_->clone()}
{}

EMANE::Controls::FlowControlControlMessage::FlowControlControlMessage(std::uint16_t u16Tokens):
  ControlMessage{IDENTIFIER},
  pImpl_{new Implementation{u16Tokens}}
{}

EMANE::Controls::FlowControlControlMessage::~FlowControlControlMessage()
{}

std::uint16_t EMANE::Controls::FlowControlControlMessage::getTokens() const
{
  return pImpl_->getTokens();
}


EMANE::Serialization EMANE::Controls::FlowControlControlMessage::serialize() const
{
  Serialization serialization;

  EMANEMessage::FlowControlControlMessage msg;
  msg.set_tokens(pImpl_->getTokens());

  if(!msg.SerializeToString(&serialization))
    {
      throw SerializationException("unable to serialize FlowControlControlMessage");
    }

  return serialization;
}

EMANE::Controls::FlowControlControlMessage *
EMANE::Controls::FlowControlControlMessage::create(std::uint16_t u16Tokens)
{
  return new FlowControlControlMessage{u16Tokens};
}

EMANE::Controls::FlowControlControlMessage *
EMANE::Controls::FlowControlControlMessage::create(const Serialization & serialization)
{
  EMANEMessage::FlowControlControlMessage msg;

  if(!msg.ParseFromString(serialization))
    {
      throw SerializationException("unable to deserialize : FlowControlControlMessage");
    }

  return new FlowControlControlMessage{static_cast<std::uint16_t>(msg.tokens())};
}

EMANE::Controls::FlowControlControlMessage *
EMANE::Controls::FlowControlControlMessage::clone() const
{
  return new FlowControlControlMessage{*this};
}
