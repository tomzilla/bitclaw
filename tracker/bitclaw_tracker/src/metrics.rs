use actix_web::web::Data;
use opentelemetry::metrics::Counter;

use crate::Tracker;

#[derive(Debug)]
pub struct Instruments {
    pub announces_ok: Counter<u64>,
    pub announces_err: Counter<u64>,
}

pub fn register(tracker: &Data<Tracker>, service_name: &str) {
    let scope = opentelemetry::InstrumentationScope::builder(service_name.to_string()).build();
    let meter = opentelemetry::global::meter_with_scope(scope);

    let instruments = Instruments {
        announces_ok: meter
            .u64_counter("announces.ok")
            .with_description("Total number of successful agent registrations")
            .build(),
        announces_err: meter
            .u64_counter("announces.err")
            .with_description("Total number of failed agent registrations")
            .build(),
    };

    let _ = tracker.metrics.set(instruments);
}
