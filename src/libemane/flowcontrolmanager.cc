#include "emane/flowcontrolmanager.h"

#include <mutex>

extern "C" {
    void* emane_rs_flow_control_manager_create();
    void emane_rs_flow_control_manager_destroy(void* ptr);
    uint16_t emane_rs_flow_control_manager_start(void* ptr, uint16_t total_tokens);
    void emane_rs_flow_control_manager_stop(void* ptr);
    
    struct FfiAddTokenResult {
        uint16_t tokens_available;
        bool status;
        bool has_send_update;
        uint16_t send_update;
    };
    FfiAddTokenResult emane_rs_flow_control_manager_add_token(void* ptr, uint16_t tokens);
    
    struct FfiRemoveTokenResult {
        uint16_t tokens_available;
        bool status;
    };
    FfiRemoveTokenResult emane_rs_flow_control_manager_remove_token(void* ptr);
    
    struct FfiProcessMessageResult {
        bool has_send_update;
        uint16_t send_update;
    };
    FfiProcessMessageResult emane_rs_flow_control_manager_process_message(void* ptr, uint16_t tokens);
}

class EMANE::FlowControlManager::Implementation
{
public:
Implementation(EMANE::DownstreamTransport & transport):
  rTransport_(transport),
  pRsManager_{emane_rs_flow_control_manager_create()}
  {}

  ~Implementation()
  {
      if(pRsManager_) {
          emane_rs_flow_control_manager_destroy(pRsManager_);
      }
  }

  void start(std::uint16_t u16TotalTokensAvailable)
  {
    std::lock_guard<std::mutex> m(mutex_);
    uint16_t response = emane_rs_flow_control_manager_start(pRsManager_, u16TotalTokensAvailable);
    sendFlowControlResponseMessage(response);
  }

  void stop()
  {
    std::lock_guard<std::mutex> m(mutex_);
    emane_rs_flow_control_manager_stop(pRsManager_);
  }

  std::pair<std::uint16_t,bool> addToken(std::uint16_t u16Tokens)
  {
    std::lock_guard<std::mutex> m(mutex_);
    auto result = emane_rs_flow_control_manager_add_token(pRsManager_, u16Tokens);
    if(result.has_send_update) {
        sendFlowControlResponseMessage(result.send_update);
    }
    return {result.tokens_available, result.status};
  }

  std::pair<std::uint16_t,bool> removeToken()
  {
    std::lock_guard<std::mutex> m(mutex_);
    auto result = emane_rs_flow_control_manager_remove_token(pRsManager_);
    return {result.tokens_available, result.status};
  }

  void processFlowControlMessage(const Controls::FlowControlControlMessage * pMsg)
  {
    std::lock_guard<std::mutex> m(mutex_);
    auto result = emane_rs_flow_control_manager_process_message(pRsManager_, pMsg->getTokens());
    if(result.has_send_update) {
        sendFlowControlResponseMessage(result.send_update);
    }
  }

private:

  DownstreamTransport & rTransport_;
  std::mutex mutex_;
  void* pRsManager_;

  // precondition - mutex is locked
  void sendFlowControlResponseMessage(uint16_t tokens)
  {
    rTransport_.sendUpstreamControl({Controls::FlowControlControlMessage::create(tokens)});
  }
};


EMANE::FlowControlManager::FlowControlManager(DownstreamTransport & transport):
  pImpl_{new Implementation{transport}}
{}

EMANE::FlowControlManager::~FlowControlManager(){}

void EMANE::FlowControlManager::start(std::uint16_t u16TotalTokensAvailable)
{
  pImpl_->start(u16TotalTokensAvailable);
}

void EMANE::FlowControlManager::stop()
{
  pImpl_->stop();
}

std::pair<std::uint16_t,bool> EMANE::FlowControlManager::addToken(std::uint16_t u16Tokens)
{
  return pImpl_->addToken(u16Tokens);
}

std::pair<std::uint16_t,bool> EMANE::FlowControlManager::removeToken()
{
  return pImpl_->removeToken();
}

void EMANE::FlowControlManager::processFlowControlMessage(const Controls::FlowControlControlMessage * pMsg)
{
  pImpl_->processFlowControlMessage(pMsg);
}
