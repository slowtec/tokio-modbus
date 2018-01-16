use futures::prelude::*;
use std::io::Error;
use frame::*;

pub trait Client {
    fn read_coils(&self, Address, Quantity) -> Box<Future<Item = Vec<Coil>, Error = Error>>;
    fn read_discrete_inputs(
        &self,
        Address,
        Quantity,
    ) -> Box<Future<Item = Vec<Coil>, Error = Error>>;
    fn write_single_coil(&self, Address, Coil) -> Box<Future<Item = (), Error = Error>>;
    fn write_multiple_coils(&self, Address, &[Coil]) -> Box<Future<Item = (), Error = Error>>;
    fn read_input_registers(
        &self,
        Address,
        Quantity,
    ) -> Box<Future<Item = Vec<Word>, Error = Error>>;
    fn read_holding_registers(
        &self,
        Address,
        Quantity,
    ) -> Box<Future<Item = Vec<Word>, Error = Error>>;
    fn write_single_register(&self, Address, Word) -> Box<Future<Item = (), Error = Error>>;
    fn write_multiple_registers(&self, Address, &[Word]) -> Box<Future<Item = (), Error = Error>>;
    fn read_write_multiple_registers(
        &self,
        Address,
        Quantity,
        Address,
        &[Word],
    ) -> Box<Future<Item = Vec<Word>, Error = Error>>;
}
