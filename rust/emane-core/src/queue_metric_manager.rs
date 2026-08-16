use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct R2RIQueueMetric {
    pub queue_id: u16,
    pub queue_max_size: u32,
    pub queue_current_depth_high_water: u32,
    pub num_discards_high_water: u32,
    pub avg_delay_microseconds: u64,
}

#[derive(Default)]
struct QueueData {
    max_queue_size: u32,
    queue_high_water_mark: u32,
    num_samples: u32,
    num_discards_high_water_mark: u32,
    sum_delay_microseconds: u64,
}

impl QueueData {
    fn new(max_size: u32) -> Self {
        Self {
            max_queue_size: max_size,
            ..Default::default()
        }
    }

    fn reset(&mut self) {
        self.num_discards_high_water_mark = 0;
        self.sum_delay_microseconds = 0;
        self.num_samples = 0;
        self.queue_high_water_mark = 0;
    }
}

pub struct QueueMetricManager {
    nem_id: u16,
    queue_data_map: HashMap<u16, QueueData>,
}

impl QueueMetricManager {
    pub fn new(nem_id: u16) -> Self {
        Self {
            nem_id,
            queue_data_map: HashMap::new(),
        }
    }

    pub fn update_queue_metric(
        &mut self,
        queue_id: u16,
        max_queue_size: u32,
        current_queue_depth: u32,
        num_discards: u32,
        delay_microseconds: u64,
    ) {
        let data = self.queue_data_map.entry(queue_id).or_insert_with(QueueData::default);

        data.num_samples += 1;
        data.sum_delay_microseconds += delay_microseconds;

        if data.max_queue_size != max_queue_size {
            data.max_queue_size = max_queue_size;
            data.queue_high_water_mark = 0;
        }

        if data.queue_high_water_mark < current_queue_depth {
            data.queue_high_water_mark = current_queue_depth;
        }

        if data.num_discards_high_water_mark < num_discards {
            data.num_discards_high_water_mark = num_discards;
        }
    }

    pub fn get_queue_metrics(&mut self) -> Vec<R2RIQueueMetric> {
        let mut metrics = Vec::with_capacity(self.queue_data_map.len());

        for (&queue_id, data) in self.queue_data_map.iter_mut() {
            let avg_delay_microseconds = if data.num_samples > 0 {
                data.sum_delay_microseconds / data.num_samples as u64
            } else {
                0
            };

            metrics.push(R2RIQueueMetric {
                queue_id,
                queue_max_size: data.max_queue_size,
                queue_current_depth_high_water: data.queue_high_water_mark,
                num_discards_high_water: data.num_discards_high_water_mark,
                avg_delay_microseconds,
            });

            data.reset();
        }

        metrics
    }

    pub fn add_queue_metric(&mut self, queue_id: u16, max_queue_size: u32) -> bool {
        if self.queue_data_map.contains_key(&queue_id) {
            false
        } else {
            self.queue_data_map.insert(queue_id, QueueData::new(max_queue_size));
            true
        }
    }

    pub fn remove_queue_metric(&mut self, queue_id: u16) -> bool {
        self.queue_data_map.remove(&queue_id).is_some()
    }
}
