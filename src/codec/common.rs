use frame::*;
use bytes::{BigEndian, BufMut, Bytes, BytesMut};

impl From<Request> for Bytes {
    fn from(req: Request) -> Bytes {
        let mut data = BytesMut::new();
        use Request::*;
        data.put_u8(req_to_fn_code(&req));
        match req {
            ReadCoils(address, quantity) |
            ReadDiscreteInputs(address, quantity) |
            ReadInputRegisters(address, quantity) |
            ReadHoldingRegisters(address, quantity) => {
                data.put_u16::<BigEndian>(address);
                data.put_u16::<BigEndian>(quantity);
            }
            WriteSingleCoil(address, state) => {
                data.put_u16::<BigEndian>(address);
                data.put_u16::<BigEndian>(bool_to_coil(state));
            }
            WriteMultipleCoils(address, coils) => {
                data.put_u16::<BigEndian>(address);
                let len = coils.len();
                data.put_u16::<BigEndian>(len as u16);
                let packed_coils = pack_coils(&coils);
                data.put_u8(packed_coils.len() as u8);
                for b in packed_coils {
                    data.put_u8(b);
                }
            }
            WriteSingleRegister(address, word) => {
                data.put_u16::<BigEndian>(address);
                data.put_u16::<BigEndian>(word);
            }
            WriteMultipleRegisters(address, words) => {
                data.put_u16::<BigEndian>(address);
                let len = words.len();
                data.put_u16::<BigEndian>(len as u16);
                for w in words {
                    data.put_u16::<BigEndian>(w);
                }
            }
            ReadWriteMultipleRegisters(read_address, quantity, write_address, words) => {
                data.put_u16::<BigEndian>(read_address);
                data.put_u16::<BigEndian>(quantity);
                data.put_u16::<BigEndian>(write_address);
                for w in words {
                    data.put_u16::<BigEndian>(w);
                }
            }
            Custom(_, custom_data) => {
                for d in custom_data {
                    data.put_u8(d);
                }
            }
        }
        data.freeze()
    }
}

impl From<Response> for Bytes {
    fn from(res: Response) -> Bytes {
        let mut data = BytesMut::new();
        use Response::*;
        data.put_u8(res_to_fn_code(&res));
        match res {
            ReadCoils(coils) |
            ReadDiscreteInputs(coils) => {
                let packed_coils = pack_coils(&coils);
                data.put_u8(packed_coils.len() as u8);
                for b in packed_coils {
                    data.put_u8(b);
                }
            }
            ReadInputRegisters(registers) |
            ReadHoldingRegisters(registers) |
            ReadWriteMultipleRegisters(registers) => {
                data.put_u8((registers.len() * 2) as u8);
                for r in registers {
                    data.put_u16::<BigEndian>(r);
                }
            }
            WriteSingleCoil(address) => {
                data.put_u16::<BigEndian>(address);
            }
            WriteMultipleCoils(address, quantity) |
            WriteMultipleRegisters(address, quantity) => {
                data.put_u16::<BigEndian>(address);
                data.put_u16::<BigEndian>(quantity);
            }
            WriteSingleRegister(address, word) => {
                data.put_u16::<BigEndian>(address);
                data.put_u16::<BigEndian>(word);
            }
            Custom(_, custom_data) => {
                for d in custom_data {
                    data.put_u8(d);
                }
            }
        }
        data.freeze()
    }
}

impl From<ExceptionResponse> for Bytes {
    fn from(ex: ExceptionResponse) -> Bytes {
        let mut data = BytesMut::new();
        data.put_u8(ex.function + 0x80);
        data.put_u8(ex.exception as u8);
        data.freeze()
    }
}

impl From<Pdu> for Bytes {
    fn from(pdu: Pdu) -> Bytes {
        use Pdu::*;
        match pdu {
            Request(req) => req.into(),
            Result(res) => {
                match res {
                    Ok(res) => res.into(),
                    Err(ex) => ex.into(),
                }
            }
        }
    }
}

fn bool_to_coil(state: bool) -> u16 {
    if state { 0xFF00 } else { 0x0000 }
}

fn pack_coils(coils: &[Coil]) -> Vec<u8> {
    let bitcount = coils.len();
    let packed_size = bitcount / 8 + if bitcount % 8 > 0 { 1 } else { 0 };
    let mut res = vec![0; packed_size];
    for (i, b) in coils.iter().enumerate() {
        let v = if *b { 0b1 } else { 0b0 };
        res[(i / 8) as usize] |= v << (i % 8);
    }
    res
}

fn req_to_fn_code(req: &Request) -> u8 {
    use Request::*;
    match *req {
        ReadCoils(_, _) => 0x01,
        ReadDiscreteInputs(_, _) => 0x02,
        WriteSingleCoil(_, _) => 0x05,
        WriteMultipleCoils(_, _) => 0x0F,
        ReadInputRegisters(_, _) => 0x04,
        ReadHoldingRegisters(_, _) => 0x03,
        WriteSingleRegister(_, _) => 0x06,
        WriteMultipleRegisters(_, _) => 0x10,
        ReadWriteMultipleRegisters(_, _, _, _) => 0x17,
        Custom(code, _) => code,
    }
}

fn res_to_fn_code(res: &Response) -> u8 {
    use Response::*;
    match *res {
        ReadCoils(_) => 0x01,
        ReadDiscreteInputs(_) => 0x02,
        WriteSingleCoil(_) => 0x05,
        WriteMultipleCoils(_, _) => 0x0F,
        ReadInputRegisters(_) => 0x04,
        ReadHoldingRegisters(_) => 0x03,
        WriteSingleRegister(_, _) => 0x06,
        WriteMultipleRegisters(_, _) => 0x10,
        ReadWriteMultipleRegisters(_) => 0x17,
        Custom(code, _) => code,
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn convert_bool_to_coil() {
        assert_eq!(bool_to_coil(true), 0xFF00);
        assert_eq!(bool_to_coil(false), 0x0000);
    }

    #[test]
    fn convert_booleans_to_bytes() {
        assert_eq!(pack_coils(&[]), &[]);
        assert_eq!(pack_coils(&[true]), &[0b_1]);
        assert_eq!(pack_coils(&[false]), &[0b_0]);
        assert_eq!(pack_coils(&[true, false]), &[0b_01]);
        assert_eq!(pack_coils(&[false, true]), &[0b_10]);
        assert_eq!(pack_coils(&[true, true]), &[0b_11]);
        assert_eq!(pack_coils(&[true; 8]), &[0b_1111_1111]);
        assert_eq!(pack_coils(&[true; 9]), &[255, 1]);
        assert_eq!(pack_coils(&[false; 8]), &[0]);
        assert_eq!(pack_coils(&[false; 9]), &[0, 0]);
    }

    #[test]
    fn function_code_from_request() {
        use Request::*;
        assert_eq!(req_to_fn_code(&ReadCoils(0, 0)), 1);
        assert_eq!(req_to_fn_code(&ReadDiscreteInputs(0, 0)), 2);
        assert_eq!(req_to_fn_code(&WriteSingleCoil(0, true)), 5);
        assert_eq!(req_to_fn_code(&WriteMultipleCoils(0, vec![])), 0x0F);
        assert_eq!(req_to_fn_code(&ReadInputRegisters(0, 0)), 0x04);
        assert_eq!(req_to_fn_code(&ReadHoldingRegisters(0, 0)), 0x03);
        assert_eq!(req_to_fn_code(&WriteSingleRegister(0, 0)), 0x06);
        assert_eq!(req_to_fn_code(&WriteMultipleRegisters(0, vec![])), 0x10);
        assert_eq!(
            req_to_fn_code(&ReadWriteMultipleRegisters(0, 0, 0, vec![])),
            0x17
        );
        assert_eq!(req_to_fn_code(&Custom(88, vec![])), 88);
    }

    #[test]
    fn function_code_from_response() {
        use Response::*;
        assert_eq!(res_to_fn_code(&ReadCoils(vec![])), 1);
        assert_eq!(res_to_fn_code(&ReadDiscreteInputs(vec![])), 2);
        assert_eq!(res_to_fn_code(&WriteSingleCoil(0x0)), 5);
        assert_eq!(res_to_fn_code(&WriteMultipleCoils(0x0, 0x0)), 0x0F);
        assert_eq!(res_to_fn_code(&ReadInputRegisters(vec![])), 0x04);
        assert_eq!(res_to_fn_code(&ReadHoldingRegisters(vec![])), 0x03);
        assert_eq!(res_to_fn_code(&WriteSingleRegister(0, 0)), 0x06);
        assert_eq!(res_to_fn_code(&WriteMultipleRegisters(0, 0)), 0x10);
        assert_eq!(res_to_fn_code(&ReadWriteMultipleRegisters(vec![])), 0x17);
        assert_eq!(res_to_fn_code(&Custom(99, vec![])), 99);
    }

    #[test]
    fn exception_response_into_bytes() {
        let bytes: Bytes = ExceptionResponse {
            function: 0x03,
            exception: Exception::IllegalDataAddress,
        }.into();
        assert_eq!(bytes[0], 0x83);
        assert_eq!(bytes[1], 0x02);
    }

    #[test]
    fn pdu_into_bytes() {
        let req_pdu: Bytes = Pdu::Request(Request::ReadCoils(0x01, 5)).into();
        let res_pdu: Bytes = Pdu::Result(Ok(Response::ReadCoils(vec![]))).into();
        let ex_pdu: Bytes = Pdu::Result(Err(ExceptionResponse {
            function: 0x03,
            exception: Exception::ServerDeviceFailure,
        })).into();

        assert_eq!(req_pdu[0], 0x01);
        assert_eq!(req_pdu[1], 0x00);
        assert_eq!(req_pdu[2], 0x01);
        assert_eq!(req_pdu[3], 0x00);
        assert_eq!(req_pdu[4], 0x05);

        assert_eq!(res_pdu[0], 0x01);
        assert_eq!(res_pdu[1], 0x00);

        assert_eq!(ex_pdu[0], 0x83);
        assert_eq!(ex_pdu[1], 0x04);
    }

    mod requests {

        use super::*;

        #[test]
        fn read_coils() {
            let bytes: Bytes = Request::ReadCoils(0x12, 4).into();
            assert_eq!(bytes[0], 1);
            assert_eq!(bytes[1], 0x00);
            assert_eq!(bytes[2], 0x12);
            assert_eq!(bytes[3], 0x00);
            assert_eq!(bytes[4], 0x04);
        }

        #[test]
        fn read_discrete_inputs() {
            let bytes: Bytes = Request::ReadDiscreteInputs(0x03, 19).into();
            assert_eq!(bytes[0], 2);
            assert_eq!(bytes[1], 0x00);
            assert_eq!(bytes[2], 0x03);
            assert_eq!(bytes[3], 0x00);
            assert_eq!(bytes[4], 19);
        }

        #[test]
        fn write_single_coil() {
            let bytes: Bytes = Request::WriteSingleCoil(0x1234, true).into();
            assert_eq!(bytes[0], 5);
            assert_eq!(bytes[1], 0x12);
            assert_eq!(bytes[2], 0x34);
            assert_eq!(bytes[3], 0xFF);
            assert_eq!(bytes[4], 0x00);
        }

        #[test]
        fn write_multiple_coils() {
            let states = vec![true, false, true, true];
            let bytes: Bytes = Request::WriteMultipleCoils(0x3311, states).into();
            assert_eq!(bytes[0], 0x0F);
            assert_eq!(bytes[1], 0x33);
            assert_eq!(bytes[2], 0x11);
            assert_eq!(bytes[3], 0x00);
            assert_eq!(bytes[4], 0x04);
            assert_eq!(bytes[5], 0x01);
            assert_eq!(bytes[6], 0b_0000_1101);
        }

        #[test]
        fn read_input_registers() {
            let bytes: Bytes = Request::ReadInputRegisters(0x09, 77).into();
            assert_eq!(bytes[0], 4);
            assert_eq!(bytes[1], 0x00);
            assert_eq!(bytes[2], 0x09);
            assert_eq!(bytes[3], 0x00);
            assert_eq!(bytes[4], 0x4D);
        }

        #[test]
        fn read_holding_registers() {
            let bytes: Bytes = Request::ReadHoldingRegisters(0x09, 77).into();
            assert_eq!(bytes[0], 3);
            assert_eq!(bytes[1], 0x00);
            assert_eq!(bytes[2], 0x09);
            assert_eq!(bytes[3], 0x00);
            assert_eq!(bytes[4], 0x4D);
        }

        #[test]
        fn write_single_register() {
            let bytes: Bytes = Request::WriteSingleRegister(0x07, 0xABCD).into();
            assert_eq!(bytes[0], 6);
            assert_eq!(bytes[1], 0x00);
            assert_eq!(bytes[2], 0x07);
            assert_eq!(bytes[3], 0xAB);
            assert_eq!(bytes[4], 0xCD);
        }

        #[test]
        fn write_multiple_registers() {
            let bytes: Bytes = Request::WriteMultipleRegisters(0x06, vec![0xABCD, 0xEF12]).into();
            assert_eq!(bytes[0], 0x10);
            assert_eq!(bytes[1], 0x00);
            assert_eq!(bytes[2], 0x06);
            assert_eq!(bytes[3], 0x00);
            assert_eq!(bytes[4], 0x02);
            assert_eq!(bytes[5], 0xAB);
            assert_eq!(bytes[6], 0xCD);
            assert_eq!(bytes[7], 0xEF);
            assert_eq!(bytes[8], 0x12);
        }

        #[test]
        fn read_write_multiple_registers() {
            let data = vec![0xABCD, 0xEF12];
            let bytes: Bytes = Request::ReadWriteMultipleRegisters(0x05, 51, 0x03, data).into();
            assert_eq!(bytes[0], 0x17);
            assert_eq!(bytes[1], 0x00);
            assert_eq!(bytes[2], 0x05);
            assert_eq!(bytes[3], 0x00);
            assert_eq!(bytes[4], 0x33);
            assert_eq!(bytes[5], 0x00);
            assert_eq!(bytes[6], 0x03);
            assert_eq!(bytes[7], 0xAB);
            assert_eq!(bytes[8], 0xCD);
            assert_eq!(bytes[9], 0xEF);
            assert_eq!(bytes[10], 0x12);
        }

        #[test]
        fn custom() {
            let bytes: Bytes = Request::Custom(0x55, vec![0xCC, 0x88, 0xAA, 0xFF]).into();
            assert_eq!(bytes[0], 0x55);
            assert_eq!(bytes[1], 0xCC);
            assert_eq!(bytes[2], 0x88);
            assert_eq!(bytes[3], 0xAA);
            assert_eq!(bytes[4], 0xFF);
        }
    }

    mod responses {

        use super::*;

        #[test]
        fn read_coils() {
            let bytes: Bytes = Response::ReadCoils(vec![true, false, false, true, false]).into();
            assert_eq!(bytes[0], 1);
            assert_eq!(bytes[1], 1);
            assert_eq!(bytes[2], 0b_0000_1001);
        }

        #[test]
        fn read_discrete_inputs() {
            let bytes: Bytes = Response::ReadDiscreteInputs(vec![true, false, true, true]).into();
            assert_eq!(bytes[0], 2);
            assert_eq!(bytes[1], 1);
            assert_eq!(bytes[2], 0b_0000_1101);
        }

        #[test]
        fn write_single_coil() {
            let bytes: Bytes = Response::WriteSingleCoil(0x33).into();
            assert_eq!(bytes[0], 5);
            assert_eq!(bytes[1], 0x00);
            assert_eq!(bytes[2], 0x33);
        }

        #[test]
        fn write_multiple_coils() {
            let bytes: Bytes = Response::WriteMultipleCoils(0x3311, 5).into();
            assert_eq!(bytes[0], 0x0F);
            assert_eq!(bytes[1], 0x33);
            assert_eq!(bytes[2], 0x11);
            assert_eq!(bytes[3], 0x00);
            assert_eq!(bytes[4], 0x05);
        }

        #[test]
        fn read_input_registers() {
            let bytes: Bytes = Response::ReadInputRegisters(vec![0xAA00, 0xCCBB, 0xEEDD]).into();
            assert_eq!(bytes[0], 4);
            assert_eq!(bytes[1], 0x06);
            assert_eq!(bytes[2], 0xAA);
            assert_eq!(bytes[3], 0x00);
            assert_eq!(bytes[4], 0xCC);
            assert_eq!(bytes[5], 0xBB);
            assert_eq!(bytes[6], 0xEE);
            assert_eq!(bytes[7], 0xDD);
        }

        #[test]
        fn read_holding_registers() {
            let bytes: Bytes = Response::ReadHoldingRegisters(vec![0xAA00, 0x1111]).into();
            assert_eq!(bytes[0], 3);
            assert_eq!(bytes[1], 0x04);
            assert_eq!(bytes[2], 0xAA);
            assert_eq!(bytes[3], 0x00);
            assert_eq!(bytes[4], 0x11);
            assert_eq!(bytes[5], 0x11);
        }

        #[test]
        fn write_single_register() {
            let bytes: Bytes = Response::WriteSingleRegister(0x07, 0xABCD).into();
            assert_eq!(bytes[0], 6);
            assert_eq!(bytes[1], 0x00);
            assert_eq!(bytes[2], 0x07);
            assert_eq!(bytes[3], 0xAB);
            assert_eq!(bytes[4], 0xCD);
        }

        #[test]
        fn write_multiple_registers() {
            let bytes: Bytes = Response::WriteMultipleRegisters(0x06, 2).into();
            assert_eq!(bytes[0], 0x10);
            assert_eq!(bytes[1], 0x00);
            assert_eq!(bytes[2], 0x06);
            assert_eq!(bytes[3], 0x00);
            assert_eq!(bytes[4], 0x02);
        }

        #[test]
        fn read_write_multiple_registers() {
            let bytes: Bytes = Response::ReadWriteMultipleRegisters(vec![0x1234]).into();
            assert_eq!(bytes[0], 0x17);
            assert_eq!(bytes[1], 0x02);
            assert_eq!(bytes[2], 0x12);
            assert_eq!(bytes[3], 0x34);
        }

        #[test]
        fn custom() {
            let bytes: Bytes = Response::Custom(0x55, vec![0xCC, 0x88, 0xAA, 0xFF]).into();
            assert_eq!(bytes[0], 0x55);
            assert_eq!(bytes[1], 0xCC);
            assert_eq!(bytes[2], 0x88);
            assert_eq!(bytes[3], 0xAA);
            assert_eq!(bytes[4], 0xFF);
        }
    }
}
