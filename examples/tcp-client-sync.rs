// SPDX-FileCopyrightText: Copyright (c) 2017-2023 slowtec GmbH <post@slowtec.de>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Synchronous TCP client example

fn main() {
    use tokio_modbus::prelude::*;

    let socket_addr = "192.168.0.222:502".parse().unwrap();
    let mut ctx = sync::tcp::connect(socket_addr).unwrap();
    let buff = ctx.read_input_registers(0x1000, 7).unwrap();
    println!("Response is '{buff:?}'");
}
