// SPDX-FileCopyrightText: Copyright (c) 2017-2023 slowtec GmbH <post@slowtec.de>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Custom function client example

use std::borrow::Cow;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    use tokio_modbus::prelude::*;

    let socket_addr = "192.168.0.222:502".parse().unwrap();

    let mut ctx = tcp::connect(socket_addr).await?;

    println!("Fetching the coupler ID");
    let rsp = ctx
        .call(Request::Custom(0x66, Cow::Borrowed(&[0x11, 0x42])))
        .await?;

    match rsp {
        Response::Custom(f, rsp) => {
            println!("Result for function {f} is '{rsp:?}'");
        }
        _ => {
            panic!("unexpected result");
        }
    }

    Ok(())
}
