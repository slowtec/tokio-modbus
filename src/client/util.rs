use super::*;

use futures::{future, Future};

use std::{cell::RefCell, io::Error, rc::Rc};

/// Helper for sharing a context between multiple clients,
/// i.e. when addressing multiple slave devices in turn.
#[derive(Default)]
pub struct SharedContext {
    context: Option<Rc<RefCell<Context>>>,
}

impl SharedContext {
    /// Create an instance by wrapping an initial, optional context.
    pub fn new(initial_context: Option<Context>) -> Self {
        Self {
            context: initial_context.map(RefCell::new).map(Rc::new),
        }
    }

    /// Check if the instance is connected, i.e. if it wraps some
    /// shared context reference.
    pub fn is_connected(&self) -> bool {
        self.context.is_some()
    }

    /// Disconnect and drop the wrapped context reference.
    pub fn disconnect(&mut self) -> impl Future<Item = (), Error = Error> {
        if let Some(context) = self.context.take() {
            future::Either::A(context.borrow().disconnect())
        } else {
            future::Either::B(future::ok(()))
        }
    }

    /// Reconnect by replacing the wrapped context reference.
    pub fn reconnect(&mut self, context: Context) {
        self.context = Some(Rc::new(RefCell::new(context)));
    }

    /// Obtain a shared reference of the wrapped context.
    ///
    /// The result should only be used for subsequent requests and must
    /// not be stored. If the `SharedContext` itself is shared it might
    /// get disconnected at any time!
    pub fn share(&self) -> Result<Rc<RefCell<Context>>, Error> {
        if let Some(ref context) = self.context {
            Ok(Rc::clone(context))
        } else {
            Err(Error::new(ErrorKind::NotConnected, "No context"))
        }
    }
}

/// Factory trait for creating a new context.
pub trait NewContext {
    fn new_context(&self) -> Box<dyn Future<Item = Context, Error = Error>>;
}

/// Reconnectable environment with a shared context.
pub struct SharedEnvironment {
    shared_context: SharedContext,
    new_context: Box<dyn NewContext>,
}

impl SharedEnvironment {
    pub fn new(inital_context: Option<Context>, new_context: Box<dyn NewContext>) -> Self {
        Self {
            shared_context: SharedContext::new(inital_context),
            new_context,
        }
    }

    pub fn shared_context(&self) -> Result<Rc<RefCell<Context>>, Error> {
        self.shared_context.share()
    }
}

pub fn reconnect_shared_context(
    shared_env: &Rc<RefCell<SharedEnvironment>>,
) -> impl Future<Item = (), Error = Error> {
    let disconnected_env = Rc::clone(shared_env);
    shared_env
        .borrow_mut()
        .shared_context
        .disconnect()
        .and_then(move |()| {
            debug_assert!(!disconnected_env.borrow().shared_context.is_connected());
            let reconnected_env = Rc::clone(&disconnected_env);
            disconnected_env
                .borrow()
                .new_context
                .new_context()
                .map(move |context| {
                    reconnected_env
                        .borrow_mut()
                        .shared_context
                        .reconnect(context)
                })
        })
}
