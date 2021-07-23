use crate::metrics::meter::Meter;
use metrics::{register_counter, register_gauge};

#[derive(Clone, Debug, Default)]
pub struct NodeMetrics {
    pub inbound_throughput: InboundThroughput,
    pub epoch_counter: EpochCounter,
    pub watermark_counter: WatermarkCounter,
}

impl NodeMetrics {
    pub fn new(node_name: &str) -> NodeMetrics {
        NodeMetrics {
            inbound_throughput: InboundThroughput::new(node_name),
            epoch_counter: EpochCounter::new(node_name),
            watermark_counter: WatermarkCounter::new(node_name),
        }
    }
}
pub struct SourceMetrics {
    pub incoming_message_rate: IncomingMessageRate,
    pub error_counter: ErrorCounter,
}
impl SourceMetrics {
    pub fn new(source_node_name: &str) -> SourceMetrics {
        SourceMetrics {
            incoming_message_rate: IncomingMessageRate::new(source_node_name),
            error_counter: ErrorCounter::new(source_node_name),
        }
    }
}

pub trait MetricValue {
    fn get_value(&mut self) -> f64;
    fn update_value(&mut self, value: u64);
}

#[derive(Clone, Debug, Default)]
pub struct InboundThroughput {
    meter: Meter,
}

impl MetricValue for InboundThroughput {
    fn get_value(&mut self) -> f64 {
        self.meter.get_one_min_rate()
    }

    fn update_value(&mut self, value: u64) {
        self.meter.mark_n(value)
    }
}

impl InboundThroughput {
    pub fn new(node_name: &str) -> InboundThroughput {
        register_gauge!(format!("{}_{}", node_name, "inbound_throughput"));
        InboundThroughput {
            meter: Meter::new(),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct EpochCounter {
    counter_value: u64,
}

impl EpochCounter {
    pub fn new(node_name: &str) -> EpochCounter {
        register_counter!(format!("{}_{}", node_name, "epoch_counter"));
        EpochCounter { counter_value: 0 }
    }
}

impl MetricValue for EpochCounter {
    fn get_value(&mut self) -> f64 {
        self.counter_value as f64
    }

    fn update_value(&mut self, value: u64) {
        self.counter_value += value
    }
}

#[derive(Clone, Debug, Default)]
pub struct WatermarkCounter {
    counter_value: u64,
}

impl WatermarkCounter {
    pub fn new(node_name: &str) -> WatermarkCounter {
        register_counter!(format!("{}_{}", node_name, "watermark_counter"));
        WatermarkCounter { counter_value: 0 }
    }
}

impl MetricValue for WatermarkCounter {
    fn get_value(&mut self) -> f64 {
        self.counter_value as f64
    }

    fn update_value(&mut self, value: u64) {
        self.counter_value += value
    }
}

pub struct IncomingMessageRate {
    meter: Meter,
}

impl IncomingMessageRate {
    pub fn new(node_name: &str) -> IncomingMessageRate {
        register_gauge!(format!("{}_{}", node_name, "incoming_message_rate"));
        IncomingMessageRate {
            meter: Meter::new(),
        }
    }
}

impl MetricValue for IncomingMessageRate {
    fn get_value(&mut self) -> f64 {
        self.meter.get_one_min_rate()
    }

    fn update_value(&mut self, value: u64) {
        self.meter.mark_n(value);
    }
}

pub struct ErrorCounter {
    counter_value: u64,
}

impl ErrorCounter {
    pub fn new(source_node_name: &str) -> ErrorCounter {
        register_counter!(format!("{}_{}", source_node_name, "error_counter"));
        ErrorCounter { counter_value: 0 }
    }
}

impl MetricValue for ErrorCounter {
    fn get_value(&mut self) -> f64 {
        self.counter_value as f64
    }

    fn update_value(&mut self, value: u64) {
        self.counter_value += value;
    }
}
