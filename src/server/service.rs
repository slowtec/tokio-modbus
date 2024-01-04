// SPDX-FileCopyrightText: Copyright (c) 2017-2024 slowtec GmbH <post@slowtec.de>
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::{future::Future, ops::Deref, pin::Pin};

/// A Modbus server service.
pub trait Service {
    /// Requests handled by the service.
    type Request;

    /// Responses given by the service.
    type Response;

    /// Errors produced by the service.
    type Error;

    /// The future response value.
    type Future: Future<Output = Result<Self::Response, Self::Error>> + Send;

    /// Process the request and return the response asynchronously.
    fn call(&self, req: Self::Request) -> Self::Future;
}

impl<D> Service for D
where
    D: Deref + ?Sized,
    D::Target: Service,
{
    type Request = <D::Target as Service>::Request;
    type Response = <D::Target as Service>::Response;
    type Error = <D::Target as Service>::Error;
    type Future = <D::Target as Service>::Future;

    /// A forwarding blanket impl to support smart pointers around [`Service`].
    fn call(&self, req: Self::Request) -> Self::Future {
        self.deref().call(req)
    }
}

/// A Modbus server service that uses dynamic dispatch.
pub trait DynamicService {
    /// Requests handled by the service.
    type Request;

    /// Responses given by the service.
    type Response;

    /// Errors produced by the service.
    type Error;

    /// Process the request and return the response asynchronously.
    #[allow(clippy::type_complexity)]
    fn call(
        &self,
        req: Self::Request,
    ) -> Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;
}

/// An adapter that allows to use a [`DynamicService`] as a [`Service`].
#[derive(Debug, Clone)]
pub struct DynamicServiceAdapter<T> {
    delegate: T,
}

impl<T> Service for DynamicServiceAdapter<T>
where
    T: DynamicService,
{
    type Request = T::Request;
    type Response = T::Response;
    type Error = T::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn call(&self, req: Self::Request) -> Self::Future {
        self.delegate.call(req)
    }
}

impl<D> DynamicService for D
where
    D: Deref + ?Sized,
    D::Target: DynamicService,
{
    type Request = <D::Target as DynamicService>::Request;
    type Response = <D::Target as DynamicService>::Response;
    type Error = <D::Target as DynamicService>::Error;

    /// A forwarding blanket impl to support smart pointers around [`DynamicService`].
    fn call(
        &self,
        req: Self::Request,
    ) -> Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>> {
        self.deref().call(req)
    }
}
