use arcon::{ignore_timeout, prelude::*};
use std::sync::Arc;

#[cfg_attr(feature = "arcon_serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "unsafe_flight", derive(abomonation_derive::Abomonation))]
#[derive(Arcon, prost::Message, Copy, Clone)]
#[arcon(unsafe_ser_id = 12, reliable_ser_id = 13, version = 1)]
pub struct CustomEvent {
    #[prost(uint64, tag = "1")]
    pub id: u64,
}

#[derive(Default)]
pub struct MyOperator;

impl Operator for MyOperator {
    type IN = u64;
    type OUT = CustomEvent;
    type TimerState = ArconNever;
    type OperatorState = EmptyState;
    type ElementIterator = std::iter::Once<ArconElement<Self::OUT>>;

    fn on_start(
        &mut self,
        _ctx: &mut OperatorContext<Self::TimerState, Self::OperatorState>,
    ) -> ArconResult<()> {
        #[cfg(feature = "metrics")]
        _ctx.register_gauge("custom_gauge");

        #[cfg(feature = "metrics")]
        _ctx.register_gauge("custom_counter");

        Ok(())
    }
    fn handle_element(
        &mut self,
        element: ArconElement<Self::IN>,
        _ctx: &mut OperatorContext<Self::TimerState, Self::OperatorState>,
    ) -> ArconResult<Self::ElementIterator> {
        let custom_event = CustomEvent { id: element.data };

        #[cfg(feature = "metrics")]
        _ctx.update_gauge("custom_gauge", 1.0);

        #[cfg(feature = "metrics")]
        _ctx.increment_counter("custom_counter");

        Ok(std::iter::once(ArconElement {
            data: custom_event,
            timestamp: element.timestamp,
        }))
    }

    ignore_timeout!();
}

#[derive(Default)]
pub struct TimerOperator;

impl Operator for TimerOperator {
    type IN = CustomEvent;
    type OUT = CustomEvent;
    type TimerState = u64;
    type OperatorState = EmptyState;
    type ElementIterator = std::iter::Once<ArconElement<Self::OUT>>;

    fn handle_element(
        &mut self,
        element: ArconElement<Self::IN>,
        ctx: &mut OperatorContext<Self::TimerState, Self::OperatorState>,
    ) -> ArconResult<Self::ElementIterator> {
        let current_time = ctx.current_time()?;
        let key = element.data.get_key();
        let time = current_time + 1000;

        if let Err(err) = ctx.schedule_at(key, time, element.data.id)? {
            error!(ctx.log(), "Failed to schedule timer with err {}", err);
        }

        Ok(std::iter::once(element))
    }

    fn handle_timeout(
        &mut self,
        timeout: Self::TimerState,
        ctx: &mut OperatorContext<Self::TimerState, Self::OperatorState>,
    ) -> ArconResult<Option<Self::ElementIterator>> {
        info!(ctx.log(), "Got a timer timeout for {:?}", timeout);
        Ok(None)
    }
}

fn main() {
    let mut app = Application::default()
        .iterator(0u64..10000000, |conf| {
            conf.set_timestamp_extractor(|x: &u64| *x);
        })
        .operator(OperatorBuilder {
            operator: Arc::new(|| MyOperator),
            state: Arc::new(|_| EmptyState),
            conf: Default::default(),
        })
        .operator(OperatorBuilder {
            operator: Arc::new(|| TimerOperator),
            state: Arc::new(|_| EmptyState),
            conf: Default::default(),
        })
        .build();

    app.start();
    app.await_termination();
}
