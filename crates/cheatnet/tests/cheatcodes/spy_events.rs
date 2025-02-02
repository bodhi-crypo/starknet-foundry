use crate::common::{
    call_contract, deploy_contract, deploy_wrapper, felt_selector_from_name, get_contracts,
    state::{build_runtime_state, create_cached_state},
};
use blockifier::state::cached_state::{CachedState, GlobalContractCache};
use cairo_felt::{felt_str, Felt252};
use cairo_lang_starknet::contract::starknet_keccak;
use cairo_vm::hint_processor::hint_processor_utils::felt_to_usize;
use cheatnet::{
    constants::build_testing_state,
    forking::state::ForkStateReader,
    runtime_extensions::forge_runtime_extension::cheatcodes::{
        declare::declare,
        spy_events::{Event, SpyTarget},
    },
    state::{CheatnetState, ExtendedStateReader},
};
use conversions::IntoConv;
use starknet_api::block::BlockNumber;
use std::vec;
use tempfile::TempDir;

pub fn felt_vec_to_event_vec(felts: &[Felt252]) -> Vec<Event> {
    let mut events = vec![];
    let mut i = 0;
    while i < felts.len() {
        let from = felts[i].clone().into_();
        let keys_length = &felts[i + 1];
        let keys = &felts[i + 2..i + 2 + felt_to_usize(keys_length).unwrap()];
        let data_length = &felts[i + 2 + felt_to_usize(keys_length).unwrap()];
        let data = &felts[i + 2 + felt_to_usize(keys_length).unwrap() + 1
            ..i + 2
                + felt_to_usize(keys_length).unwrap()
                + 1
                + felt_to_usize(data_length).unwrap()];

        events.push(Event {
            from,
            keys: Vec::from(keys),
            data: Vec::from(data),
        });

        i = i + 2 + felt_to_usize(keys_length).unwrap() + 1 + felt_to_usize(data_length).unwrap();
    }

    events
}

#[test]
fn spy_events_complex() {
    let mut cached_state = create_cached_state();
    let mut cheatnet_state = CheatnetState::default();
    let mut runtime_state = build_runtime_state(&mut cheatnet_state);

    let contract_address = deploy_contract(
        &mut cached_state,
        &mut runtime_state,
        "SpyEventsChecker",
        &[],
    );

    let id = runtime_state.cheatnet_state.spy_events(SpyTarget::All);

    let selector = felt_selector_from_name("emit_one_event");
    call_contract(
        &mut cached_state,
        &mut runtime_state,
        &contract_address,
        &selector,
        &[Felt252::from(123)],
    );

    let (length, serialized_events) = runtime_state
        .cheatnet_state
        .fetch_events(&Felt252::from(id));
    let events = felt_vec_to_event_vec(&serialized_events);

    assert_eq!(length, 1, "There should be one event");
    assert_eq!(
        events.len(),
        length,
        "Length after serialization should be the same"
    );
    assert_eq!(
        events[0],
        Event {
            from: contract_address,
            keys: vec![starknet_keccak("FirstEvent".as_ref()).into()],
            data: vec![Felt252::from(123)]
        },
        "Wrong event"
    );

    let selector = felt_selector_from_name("emit_one_event");
    call_contract(
        &mut cached_state,
        &mut runtime_state,
        &contract_address,
        &selector,
        &[Felt252::from(123)],
    );

    let (length, _) = runtime_state
        .cheatnet_state
        .fetch_events(&Felt252::from(id));
    assert_eq!(length, 1, "There should be one new event");

    let (length, _) = runtime_state
        .cheatnet_state
        .fetch_events(&Felt252::from(id));
    assert_eq!(length, 0, "There should be no new events");
}

#[test]
fn check_events_order() {
    let mut cached_state = create_cached_state();
    let mut cheatnet_state = CheatnetState::default();
    let mut runtime_state = build_runtime_state(&mut cheatnet_state);

    let spy_events_checker_address = deploy_contract(
        &mut cached_state,
        &mut runtime_state,
        "SpyEventsChecker",
        &[],
    );
    let spy_events_order_checker_address = deploy_contract(
        &mut cached_state,
        &mut runtime_state,
        "SpyEventsOrderChecker",
        &[],
    );

    let id = runtime_state.cheatnet_state.spy_events(SpyTarget::All);

    let selector = felt_selector_from_name("emit_and_call_another");
    call_contract(
        &mut cached_state,
        &mut runtime_state,
        &spy_events_order_checker_address,
        &selector,
        &[
            Felt252::from(123),
            Felt252::from(234),
            Felt252::from(345),
            spy_events_checker_address.into_(),
        ],
    );

    let (length, serialized_events) = runtime_state
        .cheatnet_state
        .fetch_events(&Felt252::from(id));
    let events = felt_vec_to_event_vec(&serialized_events);

    assert_eq!(length, 3, "There should be three events");
    assert_eq!(
        events[0],
        Event {
            from: spy_events_order_checker_address,
            keys: vec![starknet_keccak("SecondEvent".as_ref()).into()],
            data: vec![Felt252::from(123)]
        },
        "Wrong first event"
    );
    assert_eq!(
        events[1],
        Event {
            from: spy_events_checker_address,
            keys: vec![starknet_keccak("FirstEvent".as_ref()).into()],
            data: vec![Felt252::from(234)]
        },
        "Wrong second event"
    );
    assert_eq!(
        events[2],
        Event {
            from: spy_events_order_checker_address,
            keys: vec![starknet_keccak("ThirdEvent".as_ref()).into()],
            data: vec![Felt252::from(345)]
        },
        "Wrong third event"
    );
}

#[test]
fn check_events_captured_only_for_spied_contracts() {
    let mut cached_state = create_cached_state();
    let mut cheatnet_state = CheatnetState::default();
    let mut runtime_state = build_runtime_state(&mut cheatnet_state);

    let spy_events_checker_address = deploy_contract(
        &mut cached_state,
        &mut runtime_state,
        "SpyEventsChecker",
        &[],
    );
    let selector = felt_selector_from_name("emit_one_event");

    call_contract(
        &mut cached_state,
        &mut runtime_state,
        &spy_events_checker_address,
        &selector,
        &[Felt252::from(123)],
    );

    let id = runtime_state
        .cheatnet_state
        .spy_events(SpyTarget::One(spy_events_checker_address));
    call_contract(
        &mut cached_state,
        &mut runtime_state,
        &spy_events_checker_address,
        &selector,
        &[Felt252::from(123)],
    );

    let (length, serialized_events) = runtime_state
        .cheatnet_state
        .fetch_events(&Felt252::from(id));
    let events = felt_vec_to_event_vec(&serialized_events);

    assert_eq!(length, 1, "There should be one event");
    assert_eq!(
        events.len(),
        length,
        "Length after serialization should be the same"
    );
    assert_eq!(
        events[0],
        Event {
            from: spy_events_checker_address,
            keys: vec![starknet_keccak("FirstEvent".as_ref()).into()],
            data: vec![Felt252::from(123)]
        },
        "Wrong event"
    );
}

#[test]
fn duplicate_spies_on_one_address() {
    let mut cached_state = create_cached_state();
    let mut cheatnet_state = CheatnetState::default();
    let mut runtime_state = build_runtime_state(&mut cheatnet_state);

    let contract_address = deploy_contract(
        &mut cached_state,
        &mut runtime_state,
        "SpyEventsChecker",
        &[],
    );

    let id1 = runtime_state
        .cheatnet_state
        .spy_events(SpyTarget::One(contract_address));
    let id2 = runtime_state
        .cheatnet_state
        .spy_events(SpyTarget::One(contract_address));

    let selector = felt_selector_from_name("emit_one_event");
    call_contract(
        &mut cached_state,
        &mut runtime_state,
        &contract_address,
        &selector,
        &[Felt252::from(123)],
    );

    let (length1, serialized_events1) = runtime_state
        .cheatnet_state
        .fetch_events(&Felt252::from(id1));
    let (length2, _) = runtime_state
        .cheatnet_state
        .fetch_events(&Felt252::from(id2));
    let events1 = felt_vec_to_event_vec(&serialized_events1);

    assert_eq!(length1, 1, "There should be one event");
    assert_eq!(length2, 0, "There should be no events");
    assert_eq!(
        events1[0],
        Event {
            from: contract_address,
            keys: vec![starknet_keccak("FirstEvent".as_ref()).into()],
            data: vec![Felt252::from(123)]
        },
        "Wrong event"
    );
}

#[test]
fn library_call_emits_event() {
    let mut cached_state = create_cached_state();
    let mut cheatnet_state = CheatnetState::default();
    let mut runtime_state = build_runtime_state(&mut cheatnet_state);

    let contracts = get_contracts();
    let class_hash = declare(&mut cached_state, "SpyEventsChecker", &contracts).unwrap();
    let contract_address = deploy_contract(
        &mut cached_state,
        &mut runtime_state,
        "SpyEventsLibCall",
        &[],
    );

    let id = runtime_state.cheatnet_state.spy_events(SpyTarget::All);

    let selector = felt_selector_from_name("call_lib_call");
    call_contract(
        &mut cached_state,
        &mut runtime_state,
        &contract_address,
        &selector,
        &[Felt252::from(123), class_hash.into_()],
    );

    let (length, serialized_events) = runtime_state
        .cheatnet_state
        .fetch_events(&Felt252::from(id));
    let events = felt_vec_to_event_vec(&serialized_events);

    assert_eq!(length, 1, "There should be one event");
    assert_eq!(
        events[0],
        Event {
            from: contract_address,
            keys: vec![starknet_keccak("FirstEvent".as_ref()).into()],
            data: vec![Felt252::from(123)]
        },
        "Wrong event"
    );
}

#[test]
fn event_emitted_in_constructor() {
    let mut cached_state = create_cached_state();
    let mut cheatnet_state = CheatnetState::default();
    let mut runtime_state = build_runtime_state(&mut cheatnet_state);

    let id = runtime_state.cheatnet_state.spy_events(SpyTarget::All);

    let contract_address = deploy_contract(
        &mut cached_state,
        &mut runtime_state,
        "ConstructorSpyEventsChecker",
        &[Felt252::from(123)],
    );

    let (length, serialized_events) = runtime_state
        .cheatnet_state
        .fetch_events(&Felt252::from(id));
    let events = felt_vec_to_event_vec(&serialized_events);

    assert_eq!(length, 1, "There should be one event");
    assert_eq!(
        events.len(),
        length,
        "Length after serialization should be the same"
    );
    assert_eq!(
        events[0],
        Event {
            from: contract_address,
            keys: vec![starknet_keccak("FirstEvent".as_ref()).into()],
            data: vec![Felt252::from(123)]
        },
        "Wrong event"
    );
}

#[test]
fn check_if_there_is_no_interference() {
    let mut cached_state = create_cached_state();
    let mut cheatnet_state = CheatnetState::default();
    let mut runtime_state = build_runtime_state(&mut cheatnet_state);

    let contracts = get_contracts();

    let class_hash = declare(&mut cached_state, "SpyEventsChecker", &contracts).unwrap();

    let spy_events_checker_address =
        deploy_wrapper(&mut cached_state, &mut runtime_state, &class_hash, &[]).unwrap();
    let other_spy_events_checker_address =
        deploy_wrapper(&mut cached_state, &mut runtime_state, &class_hash, &[]).unwrap();

    let id1 = runtime_state
        .cheatnet_state
        .spy_events(SpyTarget::One(spy_events_checker_address));
    let id2 = runtime_state
        .cheatnet_state
        .spy_events(SpyTarget::One(other_spy_events_checker_address));

    let selector = felt_selector_from_name("emit_one_event");
    call_contract(
        &mut cached_state,
        &mut runtime_state,
        &spy_events_checker_address,
        &selector,
        &[Felt252::from(123)],
    );

    let (length1, serialized_events1) = runtime_state
        .cheatnet_state
        .fetch_events(&Felt252::from(id1));
    let (length2, _) = runtime_state
        .cheatnet_state
        .fetch_events(&Felt252::from(id2));
    let events1 = felt_vec_to_event_vec(&serialized_events1);

    assert_eq!(length1, 1, "There should be one event");
    assert_eq!(length2, 0, "There should be no events");
    assert_eq!(
        events1[0],
        Event {
            from: spy_events_checker_address,
            keys: vec![starknet_keccak("FirstEvent".as_ref()).into()],
            data: vec![Felt252::from(123)]
        },
        "Wrong event"
    );
}

#[test]
fn test_nested_calls() {
    let mut cached_state = create_cached_state();
    let mut cheatnet_state = CheatnetState::default();
    let mut runtime_state = build_runtime_state(&mut cheatnet_state);

    let spy_events_checker_address = deploy_contract(
        &mut cached_state,
        &mut runtime_state,
        "SpyEventsChecker",
        &[],
    );

    let contracts = get_contracts();

    let class_hash = declare(&mut cached_state, "SpyEventsCheckerProxy", &contracts).unwrap();

    let spy_events_checker_proxy_address = deploy_wrapper(
        &mut cached_state,
        &mut runtime_state,
        &class_hash,
        &[spy_events_checker_address.into_()],
    )
    .unwrap();
    let spy_events_checker_top_proxy_address = deploy_wrapper(
        &mut cached_state,
        &mut runtime_state,
        &class_hash,
        &[spy_events_checker_proxy_address.into_()],
    )
    .unwrap();

    let id = runtime_state.cheatnet_state.spy_events(SpyTarget::All);

    let selector = felt_selector_from_name("emit_one_event");
    call_contract(
        &mut cached_state,
        &mut runtime_state,
        &spy_events_checker_top_proxy_address,
        &selector,
        &[Felt252::from(123)],
    );

    let (length, serialized_events) = runtime_state
        .cheatnet_state
        .fetch_events(&Felt252::from(id));
    let events = felt_vec_to_event_vec(&serialized_events);

    assert_eq!(length, 3, "There should be three events");
    assert_eq!(
        events[0],
        Event {
            from: spy_events_checker_top_proxy_address,
            keys: vec![starknet_keccak("FirstEvent".as_ref()).into()],
            data: vec![Felt252::from(123)]
        },
        "Wrong first event"
    );
    assert_eq!(
        events[1],
        Event {
            from: spy_events_checker_proxy_address,
            keys: vec![starknet_keccak("FirstEvent".as_ref()).into()],
            data: vec![Felt252::from(123)]
        },
        "Wrong second event"
    );
    assert_eq!(
        events[2],
        Event {
            from: spy_events_checker_address,
            keys: vec![starknet_keccak("FirstEvent".as_ref()).into()],
            data: vec![Felt252::from(123)]
        },
        "Wrong third event"
    );
}

#[test]
fn use_multiple_spies() {
    let mut cached_state = create_cached_state();
    let mut cheatnet_state = CheatnetState::default();
    let mut runtime_state = build_runtime_state(&mut cheatnet_state);

    let spy_events_checker_address = deploy_contract(
        &mut cached_state,
        &mut runtime_state,
        "SpyEventsChecker",
        &[],
    );

    let contracts = get_contracts();

    let class_hash = declare(&mut cached_state, "SpyEventsCheckerProxy", &contracts).unwrap();

    let spy_events_checker_proxy_address = deploy_wrapper(
        &mut cached_state,
        &mut runtime_state,
        &class_hash,
        &[spy_events_checker_address.into_()],
    )
    .unwrap();
    let spy_events_checker_top_proxy_address = deploy_wrapper(
        &mut cached_state,
        &mut runtime_state,
        &class_hash,
        &[spy_events_checker_proxy_address.into_()],
    )
    .unwrap();

    let id1 = runtime_state
        .cheatnet_state
        .spy_events(SpyTarget::One(spy_events_checker_address));
    let id2 = runtime_state
        .cheatnet_state
        .spy_events(SpyTarget::One(spy_events_checker_proxy_address));
    let id3 = runtime_state
        .cheatnet_state
        .spy_events(SpyTarget::One(spy_events_checker_top_proxy_address));

    let selector = felt_selector_from_name("emit_one_event");
    call_contract(
        &mut cached_state,
        &mut runtime_state,
        &spy_events_checker_top_proxy_address,
        &selector,
        &[Felt252::from(123)],
    );

    let (length1, serialized_events1) = runtime_state
        .cheatnet_state
        .fetch_events(&Felt252::from(id1));
    let (length2, serialized_events2) = runtime_state
        .cheatnet_state
        .fetch_events(&Felt252::from(id2));
    let (length3, serialized_events3) = runtime_state
        .cheatnet_state
        .fetch_events(&Felt252::from(id3));
    let events1 = felt_vec_to_event_vec(&serialized_events1);
    let events2 = felt_vec_to_event_vec(&serialized_events2);
    let events3 = felt_vec_to_event_vec(&serialized_events3);

    assert_eq!(length1, 1, "There should be one event");
    assert_eq!(length2, 1, "There should be one event");
    assert_eq!(length3, 1, "There should be one event");

    assert_eq!(
        events1[0],
        Event {
            from: spy_events_checker_address,
            keys: vec![starknet_keccak("FirstEvent".as_ref()).into()],
            data: vec![Felt252::from(123)]
        },
        "Wrong spy_events_checker event"
    );
    assert_eq!(
        events2[0],
        Event {
            from: spy_events_checker_proxy_address,
            keys: vec![starknet_keccak("FirstEvent".as_ref()).into()],
            data: vec![Felt252::from(123)]
        },
        "Wrong spy_events_checker_proxy event"
    );
    assert_eq!(
        events3[0],
        Event {
            from: spy_events_checker_top_proxy_address,
            keys: vec![starknet_keccak("FirstEvent".as_ref()).into()],
            data: vec![Felt252::from(123)]
        },
        "Wrong spy_events_checker_top_proxy event"
    );
}

#[test]
fn test_emitted_by_emit_events_syscall() {
    let mut cached_state = create_cached_state();
    let mut cheatnet_state = CheatnetState::default();
    let mut runtime_state = build_runtime_state(&mut cheatnet_state);

    let contract_address = deploy_contract(
        &mut cached_state,
        &mut runtime_state,
        "SpyEventsChecker",
        &[],
    );

    let id = runtime_state.cheatnet_state.spy_events(SpyTarget::All);

    let selector = felt_selector_from_name("emit_event_syscall");
    call_contract(
        &mut cached_state,
        &mut runtime_state,
        &contract_address,
        &selector,
        &[Felt252::from(123), Felt252::from(456)],
    );

    let (length, serialized_events) = runtime_state
        .cheatnet_state
        .fetch_events(&Felt252::from(id));
    let events = felt_vec_to_event_vec(&serialized_events);

    assert_eq!(length, 1, "There should be one event");
    assert_eq!(
        events[0],
        Event {
            from: contract_address,
            keys: vec![Felt252::from(123)],
            data: vec![Felt252::from(456)]
        },
        "Wrong spy_events_checker event"
    );
}
#[test]
fn capture_cairo0_event() {
    let temp_dir = TempDir::new().unwrap();
    let mut cached_state = CachedState::new(
        ExtendedStateReader {
            dict_state_reader: build_testing_state(),
            fork_state_reader: Some(ForkStateReader::new(
                "http://188.34.188.184:6060/rpc/v0_6".parse().unwrap(),
                BlockNumber(960_107),
                temp_dir.path().to_str().unwrap(),
            )),
        },
        GlobalContractCache::default(),
    );
    let mut cheatnet_state = CheatnetState::default();
    let mut runtime_state = build_runtime_state(&mut cheatnet_state);

    let contract_address = deploy_contract(
        &mut cached_state,
        &mut runtime_state,
        "SpyEventsCairo0",
        &[],
    );

    let id = runtime_state.cheatnet_state.spy_events(SpyTarget::All);

    let selector = felt_selector_from_name("test_cairo0_event_collection");

    let cairo0_contract_address = felt_str!(
        "1960625ba5c435bac113ecd15af3c60e327d550fc5dbb43f07cd0875ad2f54c",
        16
    );

    call_contract(
        &mut cached_state,
        &mut runtime_state,
        &contract_address,
        &selector,
        &[cairo0_contract_address.clone()],
    );

    let (length, serialized_events) = runtime_state
        .cheatnet_state
        .fetch_events(&Felt252::from(id));

    let events = felt_vec_to_event_vec(&serialized_events);

    assert_eq!(length, 1, "There should be one event");

    assert_eq!(
        events[0],
        Event {
            from: cairo0_contract_address.into_(),
            keys: vec![starknet_keccak("my_event".as_ref()).into()],
            data: vec![Felt252::from(123_456_789)]
        },
        "Wrong spy_events_checker event"
    );
}
