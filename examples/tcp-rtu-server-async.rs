/// This example combines a rtu-server and a tcp-server with the same underlying data structure
/// You can test this on your computer by generating a virtual serial interface with
/// sudo socat -d -d pty,raw,nonblock,echo=0,link=/dev/tty-simu-server pty,raw,echo=0,link=/dev/tty-simu-client
///
use std::{
    borrow::Cow, collections::HashMap, net::SocketAddr, pin::Pin, sync::Arc, time::Duration,
};
use tokio::{net::TcpListener, sync::Mutex};
use tokio_modbus::{
    prelude::*,
    server::tcp::{accept_tcp_connection, Server},
    Address, Exception, ExceptionResponse, ExtractExceptionResponse, GetFunctionCode, Quantity,
    ResponsePdu,
};
use tokio_serial::SerialStream;

pub struct ModbusResult(Result<Response, ExceptionResponse>);

impl Into<ResponsePdu> for ModbusResult {
    fn into(self) -> ResponsePdu {
        self.0.into()
    }
}

pub struct ExampleData {
    pub input_registers: Arc<Mutex<HashMap<u16, u16>>>,
    pub holding_registers: Arc<Mutex<HashMap<u16, u16>>>,
    pub discrete_inputs: Arc<Mutex<HashMap<u16, bool>>>,
    pub coils: Arc<Mutex<HashMap<u16, bool>>>,
}

impl ExampleData {
    pub async fn read_coils(
        &self,
        address: Address,
        quantity: Quantity,
    ) -> Result<Response, Exception> {
        let coils = self.coils.lock().await;
        let mut data: Vec<bool> = Vec::new();
        for index in 0..quantity {
            match coils.get(&(address + index)) {
                Some(value) => data.push(*value),
                None => return Err(Exception::IllegalDataAddress),
            }
        }
        Ok(Response::ReadCoils(data))
    }
    pub async fn read_discrete_inputs(
        &self,
        address: Address,
        quantity: Quantity,
    ) -> Result<Response, Exception> {
        let discrete_inputs = self.discrete_inputs.lock().await;
        let mut data: Vec<bool> = Vec::new();
        for index in 0..quantity {
            match discrete_inputs.get(&(address + index)) {
                Some(value) => data.push(*value),
                None => return Err(Exception::IllegalDataAddress),
            }
        }
        Ok(Response::ReadDiscreteInputs(data))
    }

    pub async fn write_single_coil(
        &self,
        address: Address,
        new_value: bool,
    ) -> Result<Response, Exception> {
        let mut coils = self.coils.lock().await;
        match coils.get_mut(&address) {
            Some(coil) => *coil = new_value,
            None => return Err(Exception::IllegalDataAddress),
        }

        Ok(Response::WriteSingleCoil(address, new_value))
    }

    pub async fn write_multiple_coils<'a>(
        &self,
        address: Address,
        new_values: Cow<'a, [bool]>,
    ) -> Result<Response, Exception> {
        let mut coils = self.coils.lock().await;
        // first check that all coils exist
        for index in 0..new_values.len() as u16 {
            if coils.get_mut(&(address + index)).is_none() {
                return Err(Exception::IllegalDataAddress);
            }
        }
        // then write data
        for index in 0..new_values.len() {
            match coils.get_mut(&(address + index as u16)) {
                Some(coil) => *coil = *new_values.get(index).unwrap(),
                None => return Err(Exception::IllegalDataAddress),
            }
        }

        Ok(Response::WriteMultipleCoils(
            address,
            new_values.len() as u16,
        ))
    }

    pub async fn read_input_registers(
        &self,
        address: Address,
        quantity: Quantity,
    ) -> Result<Response, Exception> {
        let input_registers = self.input_registers.lock().await;
        let mut data: Vec<u16> = Vec::with_capacity(quantity as usize);
        for index in 0..quantity {
            match input_registers.get(&(address + index)) {
                Some(value) => data.push(*value),
                None => return Err(Exception::IllegalDataAddress),
            }
        }
        Ok(Response::ReadInputRegisters(data))
    }
    pub async fn read_holding_registers(
        &self,
        address: Address,
        quantity: Quantity,
    ) -> Result<Response, Exception> {
        let holding_registers = self.holding_registers.lock().await;
        let mut data: Vec<u16> = Vec::with_capacity(quantity as usize);
        for index in 0..quantity {
            match holding_registers.get(&(address + index)) {
                Some(value) => data.push(*value),
                None => return Err(Exception::IllegalDataAddress),
            }
        }
        Ok(Response::ReadHoldingRegisters(data))
    }
    pub async fn write_single_register(
        &self,
        address: Address,
        new_value: u16,
    ) -> Result<Response, Exception> {
        let mut holding_registers = self.holding_registers.lock().await;
        match holding_registers.get_mut(&address) {
            Some(value) => *value = new_value,
            None => return Err(Exception::IllegalDataAddress),
        }
        Ok(Response::WriteSingleRegister(address, new_value))
    }

    pub async fn write_multiple_registers<'a>(
        &self,
        address: Address,
        new_values: Cow<'a, [u16]>,
    ) -> Result<Response, Exception> {
        let mut holding_registers = self.holding_registers.lock().await;
        // first check that all holding registers exist
        for index in 0..new_values.len() as u16 {
            if holding_registers.get_mut(&(address + index)).is_none() {
                return Err(Exception::IllegalDataAddress);
            }
        }
        // then write data
        for index in 0..new_values.len() {
            match holding_registers.get_mut(&(address + index as u16)) {
                Some(coil) => *coil = *new_values.get(index).unwrap(),
                None => return Err(Exception::IllegalDataAddress),
            }
        }

        Ok(Response::WriteMultipleRegisters(
            address,
            new_values.len() as u16,
        ))
    }

    pub async fn restore(&self) {
        let mut input_registers = HashMap::new();
        input_registers.insert(0, 1234);
        input_registers.insert(1, 5678);
        let mut holding_registers = HashMap::new();
        holding_registers.insert(0, 10);
        holding_registers.insert(1, 20);
        holding_registers.insert(2, 30);
        holding_registers.insert(3, 40);

        let mut coils = HashMap::new();
        coils.insert(0, true);
        coils.insert(1, true);
        coils.insert(2, true);
        coils.insert(3, true);
        coils.insert(4, false);
        coils.insert(5, false);
        coils.insert(6, false);
        coils.insert(7, false);

        coils.insert(8, true);
        coils.insert(9, false);
        coils.insert(10, true);
        coils.insert(11, false);
        coils.insert(12, true);
        coils.insert(13, false);
        coils.insert(14, true);
        coils.insert(15, false);

        *self.input_registers.lock().await = input_registers;
        *self.holding_registers.lock().await = holding_registers;
        *self.coils.lock().await = coils.clone();
        *self.discrete_inputs.lock().await = coils;
    }

    fn new() -> Self {
        let data = ExampleData {
            input_registers: Arc::new(Mutex::new(HashMap::new())),
            holding_registers: Arc::new(Mutex::new(HashMap::new())),
            discrete_inputs: Arc::new(Mutex::new(HashMap::new())),
            coils: Arc::new(Mutex::new(HashMap::new())),
        };
        data
    }
}

impl ExampleData {
    pub async fn async_call(&self, req: Request<'static>) -> ModbusResult {
        let function_code = req.function_code();
        let result = match req {
            Request::ReadCoils(address, quantity) => self.read_coils(address, quantity).await,
            Request::ReadDiscreteInputs(address, quantity) => {
                self.read_discrete_inputs(address, quantity).await
            }
            Request::WriteSingleCoil(address, coil) => self.write_single_coil(address, coil).await,
            Request::WriteMultipleCoils(address, coils) => {
                self.write_multiple_coils(address, coils).await
            }
            Request::ReadInputRegisters(address, quantity) => {
                self.read_input_registers(address, quantity).await
            }
            Request::ReadHoldingRegisters(address, quantity) => {
                self.read_holding_registers(address, quantity).await
            }
            Request::WriteSingleRegister(address, word) => {
                self.write_single_register(address, word).await
            }
            Request::WriteMultipleRegisters(address, words) => {
                self.write_multiple_registers(address, words).await
            }
            _ => Err(Exception::IllegalFunction),
        };
        match result {
            Ok(result) => ModbusResult(Ok(result)),
            Err(exception) => ModbusResult(Err(ExceptionResponse {
                function: function_code,
                exception,
            })),
        }
    }
}

#[derive(Clone)]
pub struct ExampleService {
    data: Arc<ExampleData>,
}

impl ExampleService {}

impl tokio_modbus::server::Service for ExampleService {
    type Request = Request<'static>;
    type Response = ModbusResult;
    type Error = std::io::Error;
    type Future = Pin<
        Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send + Sync>,
    >;
    fn call(&self, req: Self::Request) -> Self::Future {
        let data = self.data.clone();
        Box::pin(async move {
            let response = data.async_call(req).await;
            Ok(response)
        })
    }
}

impl ExampleService {
    fn new(data: Arc<ExampleData>) -> Self {
        // Insert some test data as register values.
        Self { data }
    }
}

pub async fn tcp_server(socket_addr: SocketAddr, data: Arc<ExampleData>) -> anyhow::Result<()> {
    let listener = TcpListener::bind(socket_addr).await?;
    let server = Server::new(listener);

    let on_connected = move |stream, socket_addr| {
        let cloned_data = data.clone();
        let new_service = move |_socket_addr| Ok(Some(ExampleService::new(cloned_data.clone())));
        async move { accept_tcp_connection(stream, socket_addr, new_service) }
    };
    let on_process_error = |err| {
        eprintln!("{err}");
    };
    server.serve(&on_connected, on_process_error).await?;
    Ok(())
}

pub async fn rtu_server(tty_path: &str, data: Arc<ExampleData>) -> anyhow::Result<()> {
    let builder = tokio_serial::new(tty_path, 19200);
    let server_serial = tokio_serial::SerialStream::open(&builder).unwrap();
    let server = tokio_modbus::server::rtu::Server::new(server_serial);
    let service = ExampleService::new(data);
    server.serve_forever(service).await?;
    Ok(())
}

/// Helper function implementing reading registers from a HashMap.
pub async fn server_context(
    socket_addr: SocketAddr,
    tty_path: &str,
    data: Arc<ExampleData>,
) -> anyhow::Result<()> {
    let (_, _) = tokio::join!(
        tcp_server(socket_addr, data.clone()),
        rtu_server(tty_path, data)
    );

    Ok(())
}

async fn client_execute(mut ctx: impl Reader + Writer, client_name: &str) {
    println!("{client_name}: Reading 2 input registers...");
    let response = ctx.read_input_registers(0x00, 2).await.unwrap();
    println!("{client_name}: The result is '{response:?}'");
    assert_eq!(response, [1234, 5678]);

    println!("{client_name}: Writing 2 holding registers...");
    ctx.write_multiple_registers(0x01, &[7777, 8888])
        .await
        .unwrap();

    // Read back a block including the two registers we wrote.
    println!("{client_name}: Reading 4 holding registers...");
    let response = ctx.read_holding_registers(0x00, 4).await.unwrap();
    println!("{client_name}: The result is '{response:?}'");
    assert_eq!(response, [10, 7777, 8888, 40]);

    // Now we try to read with an invalid register address.
    // This should return a Modbus exception response with the code
    // IllegalDataAddress.
    println!("{client_name}: Reading nonexisting holding register address... (should return IllegalDataAddress)");
    let response = ctx.read_holding_registers(0x100, 1).await;
    println!("{client_name}: The result is '{response:?}'");
    assert!(response.is_err());
    let maybe_exception_response = response.err().unwrap().exception_response();
    assert!(maybe_exception_response.is_ok());
    let exception_response = maybe_exception_response.ok().unwrap();
    assert_eq!(exception_response.exception, Exception::IllegalDataAddress);

    println!("{client_name}: Done.")
}

async fn tcp_client_context(socket_addr: SocketAddr) {
    let client_name = "TCP-client";
    println!("{client_name}: Connecting client...");
    let ctx = tcp::connect(socket_addr).await.unwrap();
    client_execute(ctx, client_name).await;
}

async fn rtu_client_context(tty_path: &str) {
    let client_name = "RTU-client";
    let slave = Slave(0x17);

    println!("{client_name}: Connecting client...");
    let builder = tokio_serial::new(tty_path, 19200);
    let port = SerialStream::open(&builder).unwrap();

    let ctx = rtu::attach_slave(port, slave);
    client_execute(ctx, client_name).await;
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let socket_addr: SocketAddr = "127.0.0.1:5502".parse().unwrap();
    let socket_addr_server = socket_addr.clone();
    let data = Arc::new(ExampleData::new());
    data.restore().await;
    let data_cloned = data.clone();
    let server_handle = tokio::task::spawn(async move {
        server_context(socket_addr_server, "/dev/tty-simu-server", data_cloned).await
    });
    // Give the server some time for starting up
    tokio::time::sleep(Duration::from_secs(1)).await;

    tcp_client_context(socket_addr).await;
    data.restore().await;
    rtu_client_context("/dev/tty-simu-client").await;
    server_handle.abort();

    Ok(())
}
