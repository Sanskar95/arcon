// Copyright (c) 2020, KTH Royal Institute of Technology.
// SPDX-License-Identifier: AGPL-3.0-only

use crate::{
    application::Application,
    data::{ArconType, NodeID},
    dataflow::{
        api::OperatorBuilder,
        conf::ParallelismStrategy,
        constructor::*,
        dfg::{ChannelKind, DFGNode, DFGNodeID, DFGNodeKind, DFG},
    },
    index::EmptyState,
    prelude::AssembledApplication,
    stream::{
        node::debug::DebugNode,
        operator::{
            function::{Filter, FlatMap, Map, MapInPlace},
            Operator,
        },
    },
    util::ArconFnBounds,
};
use std::{marker::PhantomData, sync::Arc};

#[derive(Default)]
pub struct Context {
    pub(crate) dfg: DFG,
    app: Application,
    console_output: bool,
}

impl Context {
    pub fn new(app: Application) -> Self {
        Self {
            dfg: Default::default(),
            app,
            console_output: false,
        }
    }
}

/// High-level object representing a sequence of stream transformations.
pub struct Stream<IN: ArconType> {
    _marker: PhantomData<IN>,
    // ID of the node which outputs this stream.
    prev_dfg_id: DFGNodeID,
    ctx: Context,
}

impl<IN: ArconType> Stream<IN> {
    /// Adds a stateless Map operator with default configuration to the application
    ///
    /// If you need a stateful version or control over the configuration, use the operator function directly!
    ///
    /// Example
    /// ```no_run
    /// use arcon::prelude::*;
    /// let stream: Stream<u64> = Application::default()
    ///     .iterator(0..100, |conf| {
    ///         conf.set_arcon_time(ArconTime::Process);
    ///     })
    ///     .map(|x| x + 10);
    /// ```
    pub fn map<F, OUT>(self, f: F) -> Stream<OUT>
    where
        OUT: ArconType,
        F: Fn(IN) -> OUT + ArconFnBounds,
    {
        self.operator(OperatorBuilder {
            operator: Arc::new(move || Map::new(f.clone())),
            state: Arc::new(|_| EmptyState),
            conf: Default::default(),
        })
    }

    /// Adds a stateless MapInPlace operator with default configuration to the application
    ///
    /// If you need a stateful version or control over the configuration, use the operator function directly!
    ///
    /// Example
    /// ```no_run
    /// use arcon::prelude::*;
    /// let stream: Stream<u64> = Application::default()
    ///     .iterator(0..100, |conf| {
    ///         conf.set_arcon_time(ArconTime::Process);
    ///     })
    ///     .map_in_place(|x| *x += 10);
    /// ```
    pub fn map_in_place<F>(self, f: F) -> Stream<IN>
    where
        F: Fn(&mut IN) + ArconFnBounds,
    {
        self.operator(OperatorBuilder {
            operator: Arc::new(move || MapInPlace::new(f.clone())),
            state: Arc::new(|_| EmptyState),
            conf: Default::default(),
        })
    }

    /// Adds a stateless Filter operator with default configuration to the application
    ///
    /// If you need a stateful version or control over the configuration, use the operator function directly!
    ///
    /// Example
    /// ```no_run
    /// use arcon::prelude::*;
    /// let stream: Stream<u64> = Application::default()
    ///     .iterator(0..100, |conf| {
    ///         conf.set_arcon_time(ArconTime::Process);
    ///     })
    ///     .filter(|x| x < &50);
    /// ```
    pub fn filter<F>(self, f: F) -> Stream<IN>
    where
        F: Fn(&IN) -> bool + ArconFnBounds,
    {
        self.operator(OperatorBuilder {
            operator: Arc::new(move || Filter::new(f.clone())),
            state: Arc::new(|_| EmptyState),
            conf: Default::default(),
        })
    }

    /// Adds a stateless Flatmap operator with default configuration to the application
    ///
    /// If you need a stateful version or control over the configuration, use the operator function directly!
    ///
    /// Example
    /// ```no_run
    /// use arcon::prelude::*;
    /// let stream: Stream<u64> = Application::default()
    ///     .iterator(0..100, |conf| {
    ///         conf.set_arcon_time(ArconTime::Process);
    ///     })
    ///     .flatmap(|x| (0..x));
    /// ```
    pub fn flatmap<F, OUTS>(self, f: F) -> Stream<OUTS::Item>
    where
        OUTS: IntoIterator + 'static,
        OUTS::Item: ArconType,
        F: Fn(IN) -> OUTS + ArconFnBounds,
    {
        self.operator(OperatorBuilder {
            operator: Arc::new(move || FlatMap::new(f.clone())),
            state: Arc::new(|_| EmptyState),
            conf: Default::default(),
        })
    }

    /// Add an [`Operator`] to the dataflow graph
    ///
    /// Example
    /// ```no_run
    /// use arcon::prelude::*;
    /// let stream: Stream<u64> = Application::default()
    ///     .iterator(0u64..100, |conf| {
    ///         conf.set_arcon_time(ArconTime::Process);
    ///     })
    ///     .operator(OperatorBuilder {
    ///         operator: Arc::new(|| Map::new(|x| x + 10)),
    ///         state: Arc::new(|_| EmptyState),
    ///         conf: Default::default(),
    ///     });
    /// ```
    pub fn operator<OP>(mut self, builder: OperatorBuilder<OP>) -> Stream<OP::OUT>
    where
        OP: Operator<IN = IN> + 'static,
    {
        // Set up directory for the operator and create Backend
        let mut state_dir = self.ctx.app.arcon_conf().state_dir();
        let state_id = builder.state_id();
        state_dir.push(state_id.clone());
        let backend = builder.create_backend(state_dir, state_id.clone());

        let outgoing_channels = match builder.conf.parallelism_strategy {
            ParallelismStrategy::Static(num) => num,
            _ => unreachable!("Managed Parallelism not Supported yet"),
        };

        let manager_constructor = node_manager_constructor::<OP, _>(
            state_id,
            self.ctx.app.data_system.clone(),
            builder,
            backend,
            self.ctx.app.arcon_logger.clone(),
        );

        let prev_dfg_node = self.ctx.dfg.get_mut(&self.prev_dfg_id);
        let incoming_channels = prev_dfg_node.outgoing_channels;

        let next_dfg_id = self.ctx.dfg.insert(DFGNode::new(
            DFGNodeKind::Node(manager_constructor),
            outgoing_channels,
            incoming_channels,
            vec![self.prev_dfg_id],
        ));

        self.prev_dfg_id = next_dfg_id;
        Stream {
            _marker: PhantomData,
            prev_dfg_id: self.prev_dfg_id,
            ctx: self.ctx,
        }
    }

    /// Will make sure the most downstream Node will print its result to the console
    ///
    /// Note that if the Application has been configured with a debug node, it will take precedence.
    #[allow(clippy::wrong_self_convention)]
    pub fn to_console(mut self) -> Stream<IN> {
        self.ctx.console_output = true;

        Stream {
            _marker: PhantomData,
            prev_dfg_id: self.prev_dfg_id,
            ctx: self.ctx,
        }
    }

    /// Builds the Dataflow graph
    ///
    /// Returns a [`AssembledApplication`] where all runtime components
    /// have been conneted and started.
    ///
    /// Note that this method only builds the application. In order
    /// to start it, see the following [method](AssembledApplication::start).
    pub fn build(mut self) -> AssembledApplication {
        let mut target_nodes: Option<Vec<Arc<dyn std::any::Any + Send + Sync>>> = None;

        for dfg_node in self.ctx.dfg.graph.into_iter().rev() {
            match dfg_node.kind {
                DFGNodeKind::Source(channel_kind, source_manager_cons) => {
                    let nodes = target_nodes.take().unwrap();
                    let source_manager =
                        source_manager_cons(nodes, channel_kind, &mut self.ctx.app);

                    self.ctx.app.source_manager = Some(source_manager);
                }
                DFGNodeKind::Node(manager_cons) => {
                    let (channel_kind, components) = {
                        match target_nodes {
                            Some(comps) => (dfg_node.channel_kind, comps),
                            None => {
                                // At the end of the graph....
                                if self.ctx.app.debug_node_enabled() {
                                    let node: DebugNode<IN> = DebugNode::new();
                                    self.ctx.app.create_debug_node(node);
                                }

                                match self.ctx.app.abstract_debug_node {
                                    Some(ref debug_node) => {
                                        (ChannelKind::Forward, vec![debug_node.clone()])
                                    }
                                    None => (
                                        if self.ctx.console_output {
                                            ChannelKind::Console
                                        } else {
                                            ChannelKind::Mute
                                        },
                                        vec![],
                                    ),
                                }
                            }
                        }
                    };

                    // Create expected incoming channels ids
                    let in_channels: Vec<NodeID> = (0..dfg_node.ingoing_channels)
                        .map(|i| NodeID::new(i as u32))
                        .collect();

                    let nodes =
                        manager_cons(in_channels, components, channel_kind, &mut self.ctx.app);

                    target_nodes = Some(nodes);
                }
            }
        }
        AssembledApplication::new(self.ctx.app)
    }

    pub(crate) fn new(ctx: Context) -> Self {
        Self {
            _marker: PhantomData,
            prev_dfg_id: DFGNodeID(0),
            ctx,
        }
    }
}
