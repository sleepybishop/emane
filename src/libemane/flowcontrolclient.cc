#include <cstdint>
#include "emane/flowcontrolclient.h"

extern "C" {
    void* emane_rs_flow_control_client_new(void* transport_ptr, void (*send_cb)(void*, uint16_t));
    void emane_rs_flow_control_client_free(void* ptr);
    void emane_rs_flow_control_client_start(void* ptr);
    void emane_rs_flow_control_client_stop(void* ptr);
    uint16_t emane_rs_flow_control_client_remove_token(void* ptr, bool* status);
    void emane_rs_flow_control_client_process_message(void* ptr, uint16_t tokens);
}

class EMANE::FlowControlClient::Implementation
{
public:
  Implementation(EMANE::UpstreamTransport & transport):
    pState_{nullptr}
  {
      pState_ = emane_rs_flow_control_client_new(&transport, [](void* ctx, uint16_t tokens) {
          auto transport = static_cast<EMANE::UpstreamTransport*>(ctx);
          transport->sendDownstreamControl({Controls::FlowControlControlMessage::create(tokens)});
      });
  }

  ~Implementation()
  {
      emane_rs_flow_control_client_free(pState_);
  }

  void start()
  {
      emane_rs_flow_control_client_start(pState_);
  }

  void stop()
  {
      emane_rs_flow_control_client_stop(pState_);
  }

  std::pair<std::uint16_t,bool> removeToken()
  {
      bool status = false;
      uint16_t tokens = emane_rs_flow_control_client_remove_token(pState_, &status);
      return {tokens, status};
  }

  void processFlowControlMessage(const Controls::FlowControlControlMessage * pMessage)
  {
      emane_rs_flow_control_client_process_message(pState_, pMessage->getTokens());
  }

private:
  void* pState_;
};

EMANE::FlowControlClient::FlowControlClient(EMANE::UpstreamTransport & transport):
  pImpl_{new Implementation{transport}}
{}

EMANE::FlowControlClient::~FlowControlClient()
{}

void EMANE::FlowControlClient::start()
{
  pImpl_->start();
}

void EMANE::FlowControlClient::stop()
{
  pImpl_->stop();
}

std::pair<std::uint16_t,bool> EMANE::FlowControlClient::removeToken()
{
  return pImpl_->removeToken();
}

void EMANE::FlowControlClient::processFlowControlMessage(const Controls::FlowControlControlMessage * pMessage)
{
  return pImpl_->processFlowControlMessage(pMessage);
}
