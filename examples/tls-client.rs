// SPDX-FileCopyrightText: Copyright (c) 2017-2023 slowtec GmbH <post@slowtec.de>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Asynchronous TCP client example
use std::{time::Duration};
use tokio::time::{sleep_until, Instant};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    use tokio_modbus::prelude::*;

    let socket_addr = "127.0.0.1:8802".parse().unwrap();

    let mut ctx = tls::connect(socket_addr).await?;

    println!("Reading input registers");
    let data = ctx.read_input_registers(0x1000, 7).await?;
    println!("{:?}",data);

    sleep_until(Instant::now() + Duration::from_secs(5)).await;

    println!("Reading input registers");
    let data = ctx.read_input_registers(0x1000, 7).await?;
    println!("{:?}",data);

    Ok(())
}
