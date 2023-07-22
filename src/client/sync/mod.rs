// SPDX-FileCopyrightText: Copyright (c) 2017-2023 slowtec GmbH <post@slowtec.de>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Synchronous Modbus client

#[cfg(feature = "rtu-sync")]
pub mod rtu;

#[cfg(feature = "tcp-sync")]
pub mod tcp;

use std::{future::Future, io::Result, time::Duration};

use futures::future::Either;

use crate::{frame::*, slave::*};

use super::{
    Client as AsyncClient, Context as AsyncContext, Reader as AsyncReader, SlaveContext,
    Writer as AsyncWriter,
};

fn block_on_with_timeout<T>(
    runtime: &tokio::runtime::Runtime,
    timeout: Option<Duration>,
    task: impl Future<Output = Result<T>>,
) -> Result<T> {
    let task = if let Some(duration) = timeout {
        Either::Left(async move {
            tokio::time::timeout(duration, task)
                .await
                .unwrap_or_else(|elapsed| {
                    Err(std::io::Error::new(std::io::ErrorKind::TimedOut, elapsed))
                })
        })
    } else {
        Either::Right(task)
    };
    runtime.block_on(task)
}

/// A transport independent synchronous client trait.
pub trait Client: SlaveContext {
    fn call(&mut self, req: Request<'_>) -> Result<Response>;
}

/// A transport independent synchronous reader trait.
///
/// The synchronous counterpart of the asynchronous [`Reader`](`crate::client::Reader`) trait.
pub trait Reader: Client {
    fn read_coils(&mut self, _: Address, _: Quantity) -> Result<Vec<Coil>>;
    fn read_discrete_inputs(&mut self, _: Address, _: Quantity) -> Result<Vec<Coil>>;
    fn read_input_registers(&mut self, _: Address, _: Quantity) -> Result<Vec<Word>>;
    fn read_holding_registers(&mut self, _: Address, _: Quantity) -> Result<Vec<Word>>;
    fn read_write_multiple_registers(
        &mut self,
        read_addr: Address,
        read_count: Quantity,
        write_addr: Address,
        write_data: &[Word],
    ) -> Result<Vec<Word>>;
}

/// A transport independent synchronous writer trait.
///
/// The synchronous counterpart of the asynchronous [`Writer`](`crate::client::Writer`) trait.
pub trait Writer: Client {
    fn write_single_coil(&mut self, _: Address, _: Coil) -> Result<()>;
    fn write_multiple_coils(&mut self, addr: Address, data: &[Coil]) -> Result<()>;
    fn write_single_register(&mut self, _: Address, _: Word) -> Result<()>;
    fn write_multiple_registers(&mut self, addr: Address, data: &[Word]) -> Result<()>;
}

/// A synchronous Modbus client context.
#[derive(Debug)]
pub struct Context {
    runtime: tokio::runtime::Runtime,
    async_ctx: AsyncContext,
    timeout: Option<Duration>,
}

impl Context {
    /// Returns the current timeout.
    pub const fn timeout(&self) -> Option<Duration> {
        self.timeout
    }

    /// Sets a timeout duration for all subsequent operations.
    ///
    /// The timeout is disabled by passing `None`.
    pub fn set_timeout(&mut self, duration: impl Into<Option<Duration>>) {
        self.timeout = duration.into();
    }

    /// Disables the timeout for all subsequent operations.
    pub fn reset_timeout(&mut self) {
        self.timeout = None;
    }
}

impl Client for Context {
    fn call(&mut self, req: Request<'_>) -> Result<Response> {
        block_on_with_timeout(&self.runtime, self.timeout, self.async_ctx.call(req))
    }
}

impl SlaveContext for Context {
    fn set_slave(&mut self, slave: Slave) {
        self.async_ctx.set_slave(slave);
    }
}

impl Reader for Context {
    fn read_coils(&mut self, addr: Address, cnt: Quantity) -> Result<Vec<Coil>> {
        block_on_with_timeout(
            &self.runtime,
            self.timeout,
            self.async_ctx.read_coils(addr, cnt),
        )
    }

    fn read_discrete_inputs(&mut self, addr: Address, cnt: Quantity) -> Result<Vec<Coil>> {
        block_on_with_timeout(
            &self.runtime,
            self.timeout,
            self.async_ctx.read_discrete_inputs(addr, cnt),
        )
    }

    fn read_input_registers(&mut self, addr: Address, cnt: Quantity) -> Result<Vec<Word>> {
        block_on_with_timeout(
            &self.runtime,
            self.timeout,
            self.async_ctx.read_input_registers(addr, cnt),
        )
    }

    fn read_holding_registers(&mut self, addr: Address, cnt: Quantity) -> Result<Vec<Word>> {
        block_on_with_timeout(
            &self.runtime,
            self.timeout,
            self.async_ctx.read_holding_registers(addr, cnt),
        )
    }

    fn read_write_multiple_registers(
        &mut self,
        read_addr: Address,
        read_count: Quantity,
        write_addr: Address,
        write_data: &[Word],
    ) -> Result<Vec<Word>> {
        block_on_with_timeout(
            &self.runtime,
            self.timeout,
            self.async_ctx
                .read_write_multiple_registers(read_addr, read_count, write_addr, write_data),
        )
    }
}

impl Writer for Context {
    fn write_single_register(&mut self, addr: Address, data: Word) -> Result<()> {
        block_on_with_timeout(
            &self.runtime,
            self.timeout,
            self.async_ctx.write_single_register(addr, data),
        )
    }

    fn write_multiple_registers(&mut self, addr: Address, data: &[Word]) -> Result<()> {
        block_on_with_timeout(
            &self.runtime,
            self.timeout,
            self.async_ctx.write_multiple_registers(addr, data),
        )
    }

    fn write_single_coil(&mut self, addr: Address, data: Coil) -> Result<()> {
        block_on_with_timeout(
            &self.runtime,
            self.timeout,
            self.async_ctx.write_single_coil(addr, data),
        )
    }

    fn write_multiple_coils(&mut self, addr: Address, data: &[Coil]) -> Result<()> {
        block_on_with_timeout(
            &self.runtime,
            self.timeout,
            self.async_ctx.write_multiple_coils(addr, data),
        )
    }
}
