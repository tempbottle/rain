use std::net::SocketAddr;
use std::fmt;
use std::rc::Rc;

use futures::Future;

use errors::Error;
use common::asycinit::AsyncInitWrapper;
use common::wrapped::WrappedRcRefCell;
use common::{ConsistencyCheck, RcSet};
use common::id::WorkerId;
use common::resources::Resources;
use super::{DataObjectRef, TaskRef};
use super::super::state::StateRef;
use errors::Result;

pub struct Worker {
    /// Unique ID, here the registration socket address.
    id: WorkerId,

    /// Assigned tasks. The task state is stored in the `Task`.
    pub(in super::super) assigned_tasks: RcSet<TaskRef>,

    /// Scheduled tasks. Superset of `assigned_tasks`.
    pub(in super::super) scheduled_tasks: RcSet<TaskRef>,

    /// State of the worker with optional error message (informative only).
    pub(in super::super) error: Option<String>,

    /// Scheduled tasks that are also ready but not yet assigned. Disjoint from
    /// `assigned_tasks`, subset of `scheduled_tasks`.
    pub(in super::super) scheduled_ready_tasks: RcSet<TaskRef>,

    // The sum of resources of scheduled tasks that may run (or are running)
    // (TODO: Generalize for Resource not only cpus)
    pub(in super::super) active_resources: u32,

    /// Obects fully located on the worker.
    pub(in super::super) located_objects: RcSet<DataObjectRef>,

    /// Objects located or assigned to appear on the worker. Superset of `located`.
    pub(in super::super) assigned_objects: RcSet<DataObjectRef>,

    /// Objects scheduled to appear here. Any objects in `located_objects` but not here
    /// are to be removed from the worker.
    pub(in super::super) scheduled_objects: RcSet<DataObjectRef>,

    /// Control interface. Optional for testing and modelling.
    pub(in super::super) control: Option<::worker_capnp::worker_control::Client>,

    data_connection: Option<AsyncInitWrapper<::worker_capnp::worker_bootstrap::Client>>,

    pub(in super::super) resources: Resources,
}

pub type WorkerRef = WrappedRcRefCell<Worker>;

impl Worker {
    #[inline]
    pub fn id(&self) -> &WorkerId {
        &self.id
    }

    /// Create a future that completes when datastore is available
    pub fn wait_for_data_connection(
        &mut self,
        worker_ref: &WorkerRef,
        state_ref: &StateRef,
    ) -> Box<Future<Item = Rc<::worker_capnp::worker_bootstrap::Client>, Error = Error>> {
        if let Some(ref mut store) = self.data_connection {
            return store.wait();
        }
        self.data_connection = Some(AsyncInitWrapper::new());
        let worker_ref = worker_ref.clone();
        let state_ref2 = state_ref.clone();
        let state = state_ref.get();
        Box::new(
            ::tokio_core::net::TcpStream::connect(&self.id, state.handle())
                .map(move |stream| {
                    stream.set_nodelay(true).unwrap();
                    let mut rpc_system = ::common::rpc::new_rpc_system(stream, None);
                    let bootstrap: ::worker_capnp::worker_bootstrap::Client =
                        rpc_system.bootstrap(::capnp_rpc::rpc_twoparty_capnp::Side::Server);
                    let bootstrap_rc = Rc::new(bootstrap);
                    state_ref2
                        .get()
                        .handle()
                        .spawn(rpc_system.map_err(|e| panic!("Rpc system error: {:?}", e)));
                    worker_ref
                        .get_mut()
                        .data_connection
                        .as_mut()
                        .unwrap()
                        .set_value(bootstrap_rc.clone());
                    bootstrap_rc
                })
                .map_err(|e| e.into()),
        )
    }
}

impl WorkerRef {
    pub fn new(
        address: SocketAddr,
        control: Option<::worker_capnp::worker_control::Client>,
        resources: Resources,
    ) -> Self {
        WorkerRef::wrap(Worker {
            id: address,
            assigned_tasks: Default::default(),
            scheduled_tasks: Default::default(),
            error: None,
            scheduled_ready_tasks: Default::default(),
            located_objects: Default::default(),
            assigned_objects: Default::default(),
            scheduled_objects: Default::default(),
            control: control,
            active_resources: 0,
            resources: resources,
            data_connection: None,
        })
    }

    /// Return the object ID in graph.
    pub fn get_id(&self) -> WorkerId {
        self.get().id
    }
}

impl ConsistencyCheck for WorkerRef {
    /// Check for state and relationships consistency. Only explores adjacent objects but still
    /// may be slow.
    fn check_consistency(&self) -> Result<()> {
        let s = self.get();

        if s.scheduled_tasks.is_empty() && s.active_resources != 0 {
            bail!(
                "Invalid active resources: active_resources = {}",
                s.active_resources
            );
        }

        // refs
        for oref in s.located_objects.iter() {
            if !oref.get().located.contains(self) {
                bail!("located_object ref {:?} inconsistency in {:?}", oref, s)
            }
        }
        for oref in s.scheduled_objects.iter() {
            if !oref.get().scheduled.contains(self) {
                bail!("scheduled_object ref {:?} inconsistency in {:?}", oref, s)
            }
        }
        for oref in s.assigned_objects.iter() {
            if !oref.get().assigned.contains(self) {
                bail!("assigned_object ref {:?} inconsistency in {:?}", oref, s)
            }
        }
        for tref in s.assigned_tasks.iter() {
            if tref.get().assigned != Some(self.clone()) {
                bail!("assigned task ref {:?} inconsistency in {:?}", tref, s)
            }
        }
        for tref in s.scheduled_tasks.iter() {
            if tref.get().scheduled != Some(self.clone()) {
                bail!("scheduled task ref {:?} inconsistency in {:?}", tref, s)
            }
        }
        for tref in s.scheduled_ready_tasks.iter() {
            if tref.get().scheduled != Some(self.clone()) {
                bail!(
                    "scheduled_ready task ref {:?} inconsistency in {:?}",
                    tref,
                    s
                )
            }
        }
        Ok(())
    }
}

impl fmt::Debug for WorkerRef {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "WorkerRef {}", self.get_id())
    }
}

impl fmt::Debug for Worker {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Worker")
            .field("id", &self.id)
            .field("tasks", &self.assigned_tasks)
            .field("located", &self.located_objects)
            .field("assigned", &self.assigned_objects)
            .field("resources", &self.resources)
            .finish()
    }
}
