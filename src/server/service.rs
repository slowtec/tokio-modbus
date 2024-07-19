// SPDX-FileCopyrightText: Copyright (c) 2017-2024 slowtec GmbH <post@slowtec.de>
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::{future::Future, ops::Deref};

/// A Modbus server service.
pub trait Service {
    /// Requests handled by the service.
    type Request;

    /// Responses sent by the service.
    type Response;

    /// Exceptional responses sent by the service.
    type Exception;

    /// The future response value.
    type Future: Future<Output = Result<Self::Response, Self::Exception>> + Send;

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
    type Exception = <D::Target as Service>::Exception;
    type Future = <D::Target as Service>::Future;

    /// A forwarding blanket impl to support smart pointers around [`Service`].
    fn call(&self, req: Self::Request) -> Self::Future {
        self.deref().call(req)
    }
}
