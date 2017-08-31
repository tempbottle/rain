use std::net::{SocketAddr};
use std::collections::HashMap;

use futures::{Future, Stream};
use tokio_core::reactor::Handle;
use tokio_core::net::{TcpListener, TcpStream};
use tokio_io::AsyncRead;
use capnp_rpc::{RpcSystem, twoparty, rpc_twoparty_capnp};

use errors::Result;
use common::id::{SessionId, WorkerId, DataObjectId, TaskId, ClientId};
use common::rpc::new_rpc_system;
use server::graph::{Graph, WorkerRef, DataObjectRef, TaskRef, SessionRef, ClientRef};
use server::rpc::ServerBootstrapImpl;
use common::wrapped::WrappedRcRefCell;

use common::resources::Resources;

pub struct State {
    // Contained objects
    pub(super) graph: Graph,

    /// Listening port and address.
    listen_address: SocketAddr,

    /// Tokio core handle.
    handle: Handle,
}

impl State {
    pub fn add_worker(&mut self,
                      address: SocketAddr,
                      control: Option<::worker_capnp::worker_control::Client>,
                      resources: Resources) -> WorkerRef {
        unimplemented!()
    }

    pub fn remove_worker(&mut self, worker: &WorkerRef) {
        unimplemented!()
    }

    pub fn add_client(&mut self, address: &SocketAddr) -> ClientRef {
        unimplemented!()
    }

    pub fn remove_client(&mut self, client: &ClientRef) { unimplemented!() }

    pub fn add_session(&mut self, client: &ClientRef) -> SessionRef {
        unimplemented!()
    }

    pub fn remove_session(&mut self, session: &SessionRef) { unimplemented!() }

    pub fn add_object(&mut self, session: &SessionRef, id: DataObjectId /* TODO: more */) -> DataObjectRef {
        unimplemented!()
    }

    pub fn remove_object(&mut self, object: &DataObjectRef) { unimplemented!() }

    pub fn unkeep_object(&mut self, object: &DataObjectRef) { unimplemented!() }

    pub fn add_task(&mut self, session: &SessionRef, id: TaskId /* TODO: more */) -> TaskRef {
        unimplemented!()
    }

    pub fn remove_task(&mut self, task: &TaskRef) { unimplemented!() }

    pub fn worker_by_id(&self, id: WorkerId) -> Result<WorkerRef> {
        match self.graph.workers.get(&id) {
            Some(w) => Ok(w.clone()),
            None => Err(format!("Worker {:?} not found", id))?,
        }
    }

    pub fn client_by_id(&self, id: ClientId) -> Result<ClientRef> {
        match self.graph.clients.get(&id) {
            Some(c) => Ok(c.clone()),
            None => Err(format!("Client {:?} not found", id))?,
        }
    }

    pub fn session_by_id(&self, id: SessionId) -> Result<SessionRef> {
        match self.graph.sessions.get(&id) {
            Some(s) => Ok(s.clone()),
            None => Err(format!("Session {:?} not found", id))?,
        }
    }

    pub fn object_by_id(&self, id: DataObjectId) -> Result<DataObjectRef> {
        match self.graph.objects.get(&id) {
            Some(o) => Ok(o.clone()),
            None => Err(format!("Object {:?} not found", id))?,
        }
    }

    pub fn task_by_id(&self, id: TaskId) -> Result<TaskRef> {
        match self.graph.tasks.get(&id) {
            Some(t) => Ok(t.clone()),
            None => Err(format!("Task {:?} not found", id))?,
        }
    }
}

/// Note: No `Drop` impl as a `State` is assumed to live forever.
pub type StateRef = WrappedRcRefCell<State>;

impl StateRef {
    pub fn new(handle: Handle, listen_address: SocketAddr) -> Self {
        Self::wrap(State {
            graph: Default::default(),
            listen_address: listen_address,
            handle: handle,
        })
    }


    // TODO: Functional cleanup of code below after structures specification


    pub fn start(&self) {
        let listen_address = self.get().listen_address;
        let handle = self.get().handle.clone();
        let listener = TcpListener::bind(&listen_address, &handle).unwrap();

        let state = self.clone();
        let future = listener
            .incoming()
            .for_each(move |(stream, addr)| {
                state.on_connection(stream, addr);
                Ok(())
            })
            .map_err(|e| {
                panic!("Listening failed {:?}", e);
            });
        handle.spawn(future);
        info!("Start listening on address={}", listen_address);
    }

    pub fn turn(&self) {
        // Now do nothing
    }

    fn on_connection(&self, stream: TcpStream, address: SocketAddr) {
        // Handle an incoming connection; spawn gate object for it

        info!("New connection from {}", address);
        stream.set_nodelay(true).unwrap();
        let bootstrap = ::server_capnp::server_bootstrap::ToClient::new(
            ServerBootstrapImpl::new(self, address),
        ).from_server::<::capnp_rpc::Server>();

        let rpc_system = new_rpc_system(stream, Some(bootstrap.client));
        self.get().handle.spawn(rpc_system.map_err(|e| {
            panic!("RPC error: {:?}", e)
        }));
    }

    #[inline]
    pub fn handle(&self) -> Handle {
        self.get().handle.clone()
    }
}
