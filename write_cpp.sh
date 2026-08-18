cat << 'EOF' > src/libemane/r2rineighbormetriccontrolmessage.cc
#include <cstdint>
/*
 * Copyright (c) 2013-2014,2016 - Adjacent Link LLC, Bridgewater,
 * New Jersey
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

#include "emane/controls/r2rineighbormetriccontrolmessage.h"
#include "radiotorouter.pb.h"

extern "C" {
  struct R2riNeighborMetricC {
      std::uint16_t id;
      std::uint64_t num_rx_frames;
      std::uint64_t num_tx_frames;
      std::uint64_t num_missed_frames;
      std::int64_t bandwidth_consumption_microsec;
      float sinr_avg_dbm;
      float sinr_stddev;
      float noise_floor_avg_dbm;
      float noise_floor_stddev;
      std::uint64_t rx_avg_data_rate_bps;
      std::uint64_t tx_avg_data_rate_bps;
  };

  void * emane_r2ri_neighbor_metric_control_message_create();
  void emane_r2ri_neighbor_metric_control_message_add_metric(void * msg_ptr, const R2riNeighborMetricC * metric);
  void * emane_r2ri_neighbor_metric_control_message_clone(void * msg_ptr);
  void emane_r2ri_neighbor_metric_control_message_destroy(void * msg_ptr);
  size_t emane_r2ri_neighbor_metric_control_message_get_metric_count(void * msg_ptr);
  void emane_r2ri_neighbor_metric_control_message_get_metric(void * msg_ptr, size_t index, R2riNeighborMetricC * metric_out);
}

class EMANE::Controls::R2RINeighborMetricControlMessage::Implementation
{
public:
  Implementation(){
      msg_ = emane_r2ri_neighbor_metric_control_message_create();
  }

  Implementation(const R2RINeighborMetrics & neighborMetrics) {
      msg_ = emane_r2ri_neighbor_metric_control_message_create();
      for(const auto & m : neighborMetrics) {
          R2riNeighborMetricC c_metric;
          c_metric.id = m.getId();
          c_metric.num_rx_frames = m.getNumRxFrames();
          c_metric.num_tx_frames = m.getNumTxFrames();
          c_metric.num_missed_frames = m.getNumMissedFrames();
          c_metric.bandwidth_consumption_microsec = m.getBandwidthConsumption().count();
          c_metric.sinr_avg_dbm = m.getSINRAvgdBm();
          c_metric.sinr_stddev = m.getSINRStddev();
          c_metric.noise_floor_avg_dbm = m.getNoiseFloorAvgdBm();
          c_metric.noise_floor_stddev = m.getNoiseFloorStddev();
          c_metric.rx_avg_data_rate_bps = m.getRxAvgDataRatebps();
          c_metric.tx_avg_data_rate_bps = m.getTxAvgDataRatebps();
          emane_r2ri_neighbor_metric_control_message_add_metric(msg_, &c_metric);
      }
  }

  Implementation(void * msg) : msg_{msg} {}

  ~Implementation() {
      if(msg_) {
          emane_r2ri_neighbor_metric_control_message_destroy(msg_);
          msg_ = nullptr;
      }
  }

  void * getMsg() const { return msg_; }

  const R2RINeighborMetrics & getNeighborMetrics() const
  {
    if (cache_valid_) return cached_metrics_;

    cached_metrics_.clear();
    size_t count = emane_r2ri_neighbor_metric_control_message_get_metric_count(msg_);
    for(size_t i = 0; i < count; ++i) {
        R2riNeighborMetricC c_metric;
        emane_r2ri_neighbor_metric_control_message_get_metric(msg_, i, &c_metric);
        cached_metrics_.push_back(R2RINeighborMetric{
            c_metric.id,
            c_metric.num_rx_frames,
            c_metric.num_tx_frames,
            c_metric.num_missed_frames,
            Microseconds{c_metric.bandwidth_consumption_microsec},
            c_metric.sinr_avg_dbm,
            c_metric.sinr_stddev,
            c_metric.noise_floor_avg_dbm,
            c_metric.noise_floor_stddev,
            c_metric.rx_avg_data_rate_bps,
            c_metric.tx_avg_data_rate_bps
        });
    }
    cache_valid_ = true;
    return cached_metrics_;
  }

  Implementation * clone() const {
      void * cloned_msg = emane_r2ri_neighbor_metric_control_message_clone(msg_);
      return new Implementation(cloned_msg);
  }

private:
  void * msg_;
  mutable R2RINeighborMetrics cached_metrics_;
  mutable bool cache_valid_ = false;
};

EMANE::Controls::R2RINeighborMetricControlMessage::
R2RINeighborMetricControlMessage(const R2RINeighborMetricControlMessage & msg):
  ControlMessage{IDENTIFIER},
  pImpl_{msg.pImpl_->clone()}
{}


EMANE::Controls::R2RINeighborMetricControlMessage::
R2RINeighborMetricControlMessage(const R2RINeighborMetrics & neighborMetrics):
  ControlMessage(IDENTIFIER),
  pImpl_{new Implementation{neighborMetrics}}{}


EMANE::Controls::R2RINeighborMetricControlMessage::~R2RINeighborMetricControlMessage()
{}

const EMANE::Controls::R2RINeighborMetrics &
EMANE::Controls::R2RINeighborMetricControlMessage::getNeighborMetrics() const
{
  return pImpl_->getNeighborMetrics();
}

EMANE::Controls::R2RINeighborMetricControlMessage *
EMANE::Controls::R2RINeighborMetricControlMessage::create(const R2RINeighborMetrics & neighborMetrics)
{
  return new R2RINeighborMetricControlMessage{neighborMetrics};
}

EMANE::Serialization EMANE::Controls::R2RINeighborMetricControlMessage::serialize() const
{
  EMANE::Serialization serialization;

  EMANEMessage::RadioToRouterNeighborMetrics msg;

  const R2RINeighborMetrics & metrics = pImpl_->getNeighborMetrics();

  R2RINeighborMetrics::const_iterator iter = metrics.begin();

  for(;iter != metrics.end(); ++iter)
    {
      EMANEMessage::RadioToRouterNeighborMetrics::NeighborMetric * pNeighborMetric =
        msg.add_metrics();

      pNeighborMetric->set_neighborid(iter->getId());
      pNeighborMetric->set_numrxframes(iter->getNumRxFrames());
      pNeighborMetric->set_numtxframes(iter->getNumTxFrames());
      pNeighborMetric->set_nummissedframes(iter->getNumMissedFrames());
      pNeighborMetric->set_bandwidthconsumption
        (std::chrono::duration_cast<DoubleSeconds>(iter->getBandwidthConsumption()).count());

      pNeighborMetric->set_sinraverage(iter->getSINRAvgdBm());
      pNeighborMetric->set_sinrstddev(iter->getSINRStddev());
      pNeighborMetric->set_noiseflooravg(iter->getNoiseFloorAvgdBm());
      pNeighborMetric->set_noisefloorstddev(iter->getNoiseFloorStddev());
      pNeighborMetric->set_rxavgdataratebps(iter->getRxAvgDataRatebps());
      pNeighborMetric->set_txavgdataratebps(iter->getTxAvgDataRatebps());
    }

  if(!msg.SerializeToString(&serialization))
    {
      throw SerializationException("unable the serialize RadioToRouterNeighborMetrics");
    }

  return serialization;
}

EMANE::Controls::R2RINeighborMetricControlMessage *
EMANE::Controls::R2RINeighborMetricControlMessage::create(const Serialization & serialization)
{
  EMANEMessage::RadioToRouterNeighborMetrics msg;

  if(!msg.ParseFromString(serialization))
    {
      throw SerializationException("unable to deserialize : R2RINeighborMetricControlMessage");
    }

  R2RINeighborMetrics metrics;

  for(const auto & neighbor : msg.metrics())
    {
      metrics.push_back(R2RINeighborMetric{static_cast<std::uint16_t>(neighbor.neighborid()),
            neighbor.numrxframes(),
            neighbor.numtxframes(),
            neighbor.nummissedframes(),
            std::chrono::duration_cast<Microseconds>(DoubleSeconds{neighbor.bandwidthconsumption()}),
            neighbor.sinraverage(),
            neighbor.sinrstddev(),
            neighbor.noiseflooravg(),
            neighbor.noisefloorstddev(),
            neighbor.rxavgdataratebps(),
            neighbor.txavgdataratebps()});
    }

  return new R2RINeighborMetricControlMessage{metrics};
}


EMANE::Controls::R2RINeighborMetricControlMessage *
EMANE::Controls::R2RINeighborMetricControlMessage::clone() const
{
  return new R2RINeighborMetricControlMessage{*this};
}
EOF
