// SPDX-FileCopyrightText: Copyright (c) 2017-2022 slowtec GmbH <post@slowtec.de>
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::{future::Future, io, rc::Rc, sync::Arc};

/// A Modbus server service.
pub trait Service {
    /// Requests handled by the service.
    type Request;

    /// Responses given by the service.
    type Response;

    /// Errors produced by the service.
    type Error;

    /// The future response value.
    type Future: Future<Output = Result<Self::Response, Self::Error>> + Send + Sync + Unpin;

    /// Process the request and return the response asynchronously.
    fn call(&self, req: Self::Request) -> Self::Future;
}

/// Creates new `Service` values.
pub trait NewService {
    /// Requests handled by the service
    type Request;

    /// Responses given by the service
    type Response;

    /// Errors produced by the service
    type Error;

    /// The `Service` value created by this factory
    type Instance: Service<Request = Self::Request, Response = Self::Response, Error = Self::Error>;

    /// Create and return a new service value.
    fn new_service(&self) -> io::Result<Self::Instance>;
}

impl<F, R> NewService for F
where
    F: Fn() -> io::Result<R>,
    R: Service,
{
    type Request = R::Request;
    type Response = R::Response;
    type Error = R::Error;
    type Instance = R;

    fn new_service(&self) -> io::Result<R> {
        (*self)()
    }
}

impl<S: NewService + ?Sized> NewService for Arc<S> {
    type Request = S::Request;
    type Response = S::Response;
    type Error = S::Error;
    type Instance = S::Instance;

    fn new_service(&self) -> io::Result<S::Instance> {
        (**self).new_service()
    }
}

impl<S: NewService + ?Sized> NewService for Rc<S> {
    type Request = S::Request;
    type Response = S::Response;
    type Error = S::Error;
    type Instance = S::Instance;

    fn new_service(&self) -> io::Result<S::Instance> {
        (**self).new_service()
    }
}

impl<S: Service + ?Sized + 'static> Service for Box<S> {
    type Request = S::Request;
    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;

    fn call(&self, request: S::Request) -> Self::Future {
        (**self).call(request)
    }
}

impl<S: Service + ?Sized + 'static> Service for Rc<S> {
    type Request = S::Request;
    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;

    fn call(&self, request: S::Request) -> Self::Future {
        (**self).call(request)
    }
}

impl<S: Service + ?Sized + 'static> Service for Arc<S> {
    type Request = S::Request;
    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;

    fn call(&self, request: S::Request) -> Self::Future {
        (**self).call(request)
    }
}
