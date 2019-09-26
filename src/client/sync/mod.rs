//! Synchronous Modbus context access

#[cfg(feature = "rtu")]
pub mod rtu;

#[cfg(feature = "tcp")]
pub mod tcp;

use super::{
    Client as AsyncClient, Context as AsyncContext, Reader as AsyncReader, SlaveContext,
    Writer as AsyncWriter,
};

use crate::frame::*;
use crate::slave::*;

use std::io::Result;

/// A transport independent synchronous client trait.
pub trait Client: SlaveContext {
    fn call(&mut self, req: Request) -> Result<Response>;
}

/// A transport independent synchronous reader trait.
pub trait Reader: Client {
    fn read_coils(&mut self, _: Address, _: Quantity) -> Result<Vec<Coil>>;
    fn read_discrete_inputs(&mut self, _: Address, _: Quantity) -> Result<Vec<Coil>>;
    fn read_input_registers(&mut self, _: Address, _: Quantity) -> Result<Vec<Word>>;
    fn read_holding_registers(&mut self, _: Address, _: Quantity) -> Result<Vec<Word>>;
    fn read_write_multiple_registers(
        &mut self,
        _: Address,
        _: Quantity,
        _: Address,
        _: &[Word],
    ) -> Result<Vec<Word>>;
}

/// A transport independent synchronous writer trait.
pub trait Writer: Client {
    fn write_single_coil(&mut self, _: Address, _: Coil) -> Result<()>;
    fn write_multiple_coils(&mut self, _: Address, _: &[Coil]) -> Result<()>;
    fn write_single_register(&mut self, _: Address, _: Word) -> Result<()>;
    fn write_multiple_registers(&mut self, _: Address, _: &[Word]) -> Result<()>;
}

/// A synchronous Modbus client context.
pub struct Context {
    core: tokio::runtime::Runtime,
    async_ctx: AsyncContext,
}

impl Client for Context {
    fn call(&mut self, req: Request) -> Result<Response> {
        self.core.block_on(self.async_ctx.call(req))
    }
}

impl SlaveContext for Context {
    fn set_slave(&mut self, slave: Slave) {
        self.async_ctx.set_slave(slave);
    }
}

impl Reader for Context {
    fn read_coils(&mut self, addr: Address, cnt: Quantity) -> Result<Vec<Coil>> {
        self.core.block_on(self.async_ctx.read_coils(addr, cnt))
    }

    fn read_discrete_inputs(&mut self, addr: Address, cnt: Quantity) -> Result<Vec<Coil>> {
        self.core
            .block_on(self.async_ctx.read_discrete_inputs(addr, cnt))
    }

    fn read_input_registers(&mut self, addr: Address, cnt: Quantity) -> Result<Vec<Word>> {
        self.core
            .block_on(self.async_ctx.read_input_registers(addr, cnt))
    }

    fn read_holding_registers(&mut self, addr: Address, cnt: Quantity) -> Result<Vec<Word>> {
        self.core
            .block_on(self.async_ctx.read_holding_registers(addr, cnt))
    }

    fn read_write_multiple_registers(
        &mut self,
        read_addr: Address,
        read_cnt: Quantity,
        write_addr: Address,
        write_data: &[Word],
    ) -> Result<Vec<Word>> {
        self.core.block_on(
            self.async_ctx
                .read_write_multiple_registers(read_addr, read_cnt, write_addr, write_data),
        )
    }
}

impl Writer for Context {
    fn write_single_register(&mut self, addr: Address, data: Word) -> Result<()> {
        self.core
            .block_on(self.async_ctx.write_single_register(addr, data))
    }

    fn write_multiple_registers(&mut self, addr: Address, data: &[Word]) -> Result<()> {
        self.core
            .block_on(self.async_ctx.write_multiple_registers(addr, data))
    }

    fn write_single_coil(&mut self, addr: Address, coil: Coil) -> Result<()> {
        self.core.block_on(self.async_ctx.write_single_coil(addr, coil))
    }

    fn write_multiple_coils(&mut self, addr: Address, coils: &[Coil]) -> Result<()> {
        self.core
            .block_on(self.async_ctx.write_multiple_coils(addr, coils))
    }
}
