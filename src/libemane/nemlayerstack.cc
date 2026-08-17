#include <cstdint>
/*
 * Copyright (c) 2013 - Adjacent Link LLC, Bridgewater, New Jersey
 * Copyright (c) 2008 - DRS CenGen, LLC, Columbia, Maryland
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
 * * Neither the name of DRS CenGen, LLC nor the names of its
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

#include "nemlayerstack.h"

extern "C" {
    void* emane_rs_nem_layer_stack_create();
    void emane_rs_nem_layer_stack_destroy(void* ptr);
    void emane_rs_nem_layer_stack_add(void* ptr, void* layer);
    void emane_rs_nem_layer_stack_connect(void* ptr, void* up, void* down);
    void emane_rs_nem_layer_stack_start(void* ptr);
    void emane_rs_nem_layer_stack_post_start(void* ptr);
    void emane_rs_nem_layer_stack_stop(void* ptr);
    void emane_rs_nem_layer_stack_destroy_layers(void* ptr);

    // C callbacks for Rust
    void emane_c_transport_set_downstream(void* transport, void* downstream) {
        if(transport && downstream) {
            static_cast<EMANE::UpstreamTransport*>(transport)->setDownstreamTransport(
                static_cast<EMANE::DownstreamTransport*>(downstream)
            );
        }
    }

    void emane_c_transport_set_upstream(void* transport, void* upstream) {
        if(transport && upstream) {
            static_cast<EMANE::DownstreamTransport*>(transport)->setUpstreamTransport(
                static_cast<EMANE::UpstreamTransport*>(upstream)
            );
        }
    }

    void emane_c_component_start(void* comp) {
        if(comp) static_cast<EMANE::Component*>(comp)->start();
    }
    
    void emane_c_component_post_start(void* comp) {
        if(comp) static_cast<EMANE::Component*>(comp)->postStart();
    }
    
    void emane_c_component_stop(void* comp) {
        if(comp) static_cast<EMANE::Component*>(comp)->stop();
    }
    
    void emane_c_component_destroy(void* comp) {
        if(comp) static_cast<EMANE::Component*>(comp)->destroy();
    }
}

EMANE::NEMLayerStack::NEMLayerStack() {
    pRsNemLayerStack_ = emane_rs_nem_layer_stack_create();
}
    
EMANE::NEMLayerStack::~NEMLayerStack() {
    if(pRsNemLayerStack_) {
        emane_rs_nem_layer_stack_destroy(pRsNemLayerStack_);
        pRsNemLayerStack_ = nullptr;
    }
}
    
void EMANE::NEMLayerStack::connectLayers(UpstreamTransport * pUpstreamTransport,
                                         DownstreamTransport * pDownstreamTransport)
{
    emane_rs_nem_layer_stack_connect(pRsNemLayerStack_, pUpstreamTransport, pDownstreamTransport);
}

void EMANE::NEMLayerStack::addLayer(std::unique_ptr<NEMLayer> & pNEMLayer)
{
    // We keep ownership in nemLayers_ so it is freed eventually
    nemLayers_.push_back(std::move(pNEMLayer));
    emane_rs_nem_layer_stack_add(pRsNemLayerStack_, nemLayers_.back().get());
}

void EMANE::NEMLayerStack::initialize(Registrar &)
{}
    
void EMANE::NEMLayerStack::configure(const ConfigurationUpdate &)
{}
    
void EMANE::NEMLayerStack::start()
{
    emane_rs_nem_layer_stack_start(pRsNemLayerStack_);
}

void EMANE::NEMLayerStack::postStart()
{
    emane_rs_nem_layer_stack_post_start(pRsNemLayerStack_);
}
    
void EMANE::NEMLayerStack::stop()
{
    emane_rs_nem_layer_stack_stop(pRsNemLayerStack_);
}
    
void EMANE::NEMLayerStack::destroy() throw()
{
    emane_rs_nem_layer_stack_destroy_layers(pRsNemLayerStack_);
}

void* EMANE::NEMLayerStack::getRsNemLayerStack() const {
    return pRsNemLayerStack_;
}
