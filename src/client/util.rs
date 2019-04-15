//! Utilities for sharing a Modbus context

use super::*;

use futures::{future, Future};

use std::{cell::RefCell, io::Error, rc::Rc};

/// Helper for sharing a context between multiple clients,
/// i.e. when addressing multiple slave devices in turn.
#[derive(Default)]
struct SharedContextHolder {
    context: Option<Rc<RefCell<Context>>>,
}

impl SharedContextHolder {
    /// Create an instance by wrapping an initial, optional context.
    fn new(initial_context: Option<Context>) -> Self {
        Self {
            context: initial_context.map(RefCell::new).map(Rc::new),
        }
    }

    /// Disconnect and drop the wrapped context reference.
    fn disconnect(&mut self) -> impl Future<Item = (), Error = Error> {
        if let Some(context) = self.context.take() {
            future::Either::A(context.borrow().disconnect())
        } else {
            future::Either::B(future::ok(()))
        }
    }

    /// Reconnect by replacing the wrapped context reference.
    fn reconnect(&mut self, context: Context) {
        self.context = Some(Rc::new(RefCell::new(context)));
    }

    pub fn is_connected(&self) -> bool {
        self.context.is_some()
    }

    fn share_context(&self) -> Option<Rc<RefCell<Context>>> {
        self.context.as_ref().map(Rc::clone)
    }
}

/// Trait for (re-)creating new contexts on demand.
///
/// Implement this trait for reconnecting a `SharedContext` on demand.
pub trait NewContext {
    /// Create a new context.
    fn new_context(&self) -> Box<dyn Future<Item = Context, Error = Error>>;
}

/// Reconnectable environment with a shared context.
pub struct SharedContext {
    shared_context: SharedContextHolder,
    new_context: Box<dyn NewContext>,
}

impl SharedContext {
    /// Create a new instance with an optional, initial context and
    /// a trait object for reconnecting the shared context on demand.
    pub fn new(inital_context: Option<Context>, new_context: Box<dyn NewContext>) -> Self {
        Self {
            shared_context: SharedContextHolder::new(inital_context),
            new_context,
        }
    }

    /// Checks if a shared context is available.
    pub fn is_connected(&self) -> bool {
        self.shared_context.is_connected()
    }

    /// Try to obtain a shared context reference. The result is `None`
    /// if no context is available, i.e. if the shared context is not
    /// connected.
    ///
    /// The result should only be used temporarily for the next
    /// asynchronous request and must not be reused later!
    pub fn share_context(&self) -> Option<Rc<RefCell<Context>>> {
        self.shared_context.share_context()
    }
}

/// Asynchronously (disconnect and) reconnect the shared context.
pub fn reconnect_shared_context(
    shared_context: &Rc<RefCell<SharedContext>>,
) -> impl Future<Item = (), Error = Error> {
    let disconnected_context = Rc::clone(shared_context);
    // The existing context needs to be disconnected first to
    // release any resources that might be reused for the new
    // context, i.e. a serial port with exclusive access.
    shared_context
        .borrow_mut()
        .shared_context
        .disconnect()
        .and_then(move |()| {
            // After disconnecting the existing context create
            // a new instance...
            debug_assert!(!disconnected_context.borrow().is_connected());
            let reconnected_context = Rc::clone(&disconnected_context);
            disconnected_context
                .borrow()
                .new_context
                .new_context()
                .map(move |context| {
                    // ...and put it into the shared context. The new
                    // context will then be used for all subsequent
                    // client requests.
                    reconnected_context
                        .borrow_mut()
                        .shared_context
                        .reconnect(context)
                })
        })
}
