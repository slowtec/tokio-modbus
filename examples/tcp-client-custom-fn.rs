#[cfg(feature = "tcp")]
pub fn main() {
    use tokio_modbus::prelude::*;

    let mut rt = tokio::runtime::Runtime::new().unwrap();
    let socket_addr = "192.168.0.222:502".parse().unwrap();

    let task = async {
        let mut ctx = tcp::connect(socket_addr).await?;

        println!("Fetching the coupler ID");
        let rsp = ctx.call(Request::Custom(0x66, vec![0x11, 0x42])).await?;

        match rsp {
            Response::Custom(f, rsp) => {
                println!("Result for function {} is '{:?}'", f, rsp);
            }
            _ => {
                panic!("unexpeted result");
            }
        }

        Result::<_, std::io::Error>::Ok(())
    };

    rt.block_on(task).unwrap();
}

#[cfg(not(feature = "tcp"))]
pub fn main() {
    println!("feature `tcp` is required to run this example");
    std::process::exit(1);
}
