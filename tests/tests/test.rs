// Copyright 2015-2017 Parity Technologies (UK) Ltd.
// This file is part of Parity.

// Parity is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Parity is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Parity.  If not, see <http://www.gnu.org/licenses/>.

extern crate ethabi;
#[macro_use]
extern crate ethabi_contract;
extern crate ethabi_derive;
extern crate ethereum_types;
extern crate rustc_hex;
extern crate solaris;
extern crate solc;

use std::fs;
use rustc_hex::FromHex;
use ethereum_types::{Address, U256};

use_contract!(
    get_sender_test,
    "contracts/GetSenderTest.abi"
);

use_contract!(
    event_log_test,
    "contracts/EventLogTest.abi"
);

use_contract!(
    constructor_test,
    "contracts/ConstructorTest.abi"
);

use_contract!(
    library_test,
    "contracts/LibraryTest.abi"
);

#[test]
fn msg_sender_should_match_value_passed_into_with_sender() {
    let mut evm = solaris::evm();

    let contract_owner_address: Address = Address::from_low_u64_be(3);

    let code_hex = include_str!("../contracts/GetSenderTest.bin");
    let code_bytes = code_hex.from_hex().unwrap();
    let _contract_address = evm.with_sender(contract_owner_address)
        .deploy(&code_bytes)
        .expect("contract deployment should succeed");

    use get_sender_test::functions;

    let sender = Address::from_low_u64_be(5);

    let result_data = evm.with_sender(sender)
        .call(functions::get_sender::encode_input(), None)
        .unwrap();

    let output: Address = functions::get_sender::decode_output(&result_data)
        .unwrap();
            
    assert_eq!(output, sender);
}

use_contract!(
    get_value_test,
    "contracts/GetValueTest.abi"
);

#[test]
fn msg_value_should_match_value_passed_into_with_value() {
    let mut evm = solaris::evm();

    let contract_owner_address: Address = Address::from_low_u64_be(3);

    let code_hex = include_str!("../contracts/GetValueTest.bin");
    let code_bytes = code_hex.from_hex().unwrap();
    let _contract_address = evm.with_sender(contract_owner_address)
        .deploy(&code_bytes)
        .expect("contract deployment should succeed");

    use get_value_test::functions;

    let value = solaris::wei::from_ether(1);

    let result_data = evm.with_value(value)
        .ensure_funds()
        .call(functions::get_value::encode_input(), None)
        .unwrap();
    
    let output: U256 = functions::get_value::decode_output(&result_data)
        .unwrap();

    assert_eq!(output, value);
}

#[test]
fn logs_should_get_collected_and_retrieved_correctly() {
    let code_hex = include_str!("../contracts/EventLogTest.bin");
    let code_bytes = code_hex.from_hex().unwrap();

    let mut evm = solaris::evm();

    let contract_owner_address: Address = Address::from_low_u64_be(3);

    let _contract_address = evm.with_sender(contract_owner_address)
        .deploy(&code_bytes)
        .expect("contract deployment should succeed");

    use event_log_test::functions;

    let first_sender_address = Address::from_low_u64_be(10);
    evm.with_sender(first_sender_address)
        .transact(functions::emit_foo::encode_input(), None)
        .unwrap();

    let second_sender_address = Address::from_low_u64_be(11);
    evm.with_sender(second_sender_address)
        .transact(functions::emit_foo::encode_input(), None)
        .unwrap();

    evm.transact(functions::emit_bar::encode_input(100), None).unwrap();
    evm.transact(functions::emit_bar::encode_input(101), None).unwrap();
    evm.transact(functions::emit_bar::encode_input(102), None).unwrap();

    // call should not show up in logs
    evm.call(functions::emit_foo::encode_input(), None)
        .unwrap();

    assert_eq!(evm.raw_logs().len(), 5);

    use event_log_test::events;

    let foo_logs: Vec<event_log_test::logs::Foo> = evm.raw_logs()
        .iter()
        .filter_map(|log| events::foo::parse_log(log.clone()).ok())
        .collect();

    assert_eq!(foo_logs.len(), 2);
    assert_eq!(Address::from(foo_logs[0].sender), first_sender_address);
    assert_eq!(Address::from(foo_logs[1].sender), second_sender_address);

    let bar_logs: Vec<event_log_test::logs::Bar> = evm.raw_logs()
        .iter()
        .filter_map(|log| events::bar::parse_log(log.clone()).ok())
        .collect();

    assert_eq!(bar_logs.len(), 3);
    assert_eq!(U256::from(bar_logs[0].value), U256::from(100));
    assert_eq!(U256::from(bar_logs[1].value), U256::from(101));
    assert_eq!(U256::from(bar_logs[2].value), U256::from(102));

    let baz_logs: Vec<event_log_test::logs::Baz> = evm.raw_logs()
        .iter()
        .filter_map(|log| events::baz::parse_log(log.clone()).ok())
        .collect();

    assert_eq!(baz_logs.len(), 0);
}

#[test]
fn value_should_match_value_passed_into_constructor() {
    let mut evm = solaris::evm();

    let contract_owner_address: Address = Address::from_low_u64_be(3);

    let code_hex = include_str!("../contracts/ConstructorTest.bin");
    let code_bytes = code_hex.from_hex().unwrap();
    let constructor_bytes = constructor_test::constructor(code_bytes, 100);
    let _contract_address = evm.with_sender(contract_owner_address)
        .deploy(&constructor_bytes)
        .expect("contract deployment should succeed");

    use constructor_test::functions;

    let result_data = evm
        .ensure_funds()
        .call(functions::get_value::encode_input(), None)
        .unwrap();
    
    let output: U256 = functions::get_value::decode_output(&result_data)
        .unwrap();

    assert_eq!(output, U256::from(100));
}

#[test]
fn deploy_contract_with_linking_library_should_succeed() {
    let mut evm = solaris::evm();

    let contract_owner_address: Address = Address::from_low_u64_be(3);

    // deploy TestLibrary
    let code_hex = include_str!("../contracts/TestLibrary.bin");
    let code_bytes = code_hex.from_hex().unwrap();
    let library_address = evm.with_sender(contract_owner_address)
        .deploy(&code_bytes)
        .expect("library deployment should succeed");

    // link to deployed library
    solc::link(
        vec![format!("test.sol:TestLibrary:{:x}", library_address)],
        "LibraryTest.bin".into(),
        concat!(env!("CARGO_MANIFEST_DIR"), "/contracts/"));

    // deploy LibraryTest
    // can't use include_str because the bytecode is updated linking to library
    let code_hex = fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/contracts/LibraryTest.bin")).unwrap();
    let code_bytes = code_hex.from_hex().unwrap();
    let _contract_address = evm.with_sender(contract_owner_address)
        .deploy(&code_bytes)
        .expect("contract deployment should succeed");

    use library_test::functions;

    let result_data = evm
        .ensure_funds()
        .call(functions::get_value_from_library::encode_input(), None)
        .unwrap();
    
    let output: U256 = functions::get_value_from_library::decode_output(&result_data)
        .unwrap();

    assert_eq!(output, U256::from(300));
}
