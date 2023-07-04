use crate::stdlib::prelude::*;

use crate::types::relocatable::Relocatable;
use felt::Felt252;
use lazy_static::lazy_static;
use num_bigint::BigUint;

#[macro_export]
macro_rules! relocatable {
    ($val1 : expr, $val2 : expr) => {
        Relocatable {
            segment_index: $val1,
            offset: $val2,
        }
    };
}

lazy_static! {
    pub static ref CAIRO_PRIME: BigUint = Felt252::prime();
}

#[macro_export]
macro_rules! any_box {
    ($val : expr) => {
        $crate::stdlib::boxed::Box::new($val) as $crate::stdlib::boxed::Box<dyn core::any::Any>
    };
}

pub fn is_subsequence<T: PartialEq>(subsequence: &[T], mut sequence: &[T]) -> bool {
    for search in subsequence {
        if let Some(index) = sequence.iter().position(|element| search == element) {
            sequence = &sequence[index + 1..];
        } else {
            return false;
        }
    }
    true
}

pub fn from_relocatable_to_indexes(relocatable: Relocatable) -> (usize, usize) {
    if relocatable.segment_index.is_negative() {
        (
            -(relocatable.segment_index + 1) as usize,
            relocatable.offset,
        )
    } else {
        (relocatable.segment_index as usize, relocatable.offset)
    }
}

#[cfg(test)]
#[macro_use]
pub mod test_utils {
    use crate::types::exec_scope::ExecutionScopes;
    use crate::types::relocatable::MaybeRelocatable;
    use crate::vm::trace::trace_entry::TraceEntry;

    #[macro_export]
    macro_rules! bigint {
        ($val : expr) => {
            Into::<num_bigint::BigInt>::into($val)
        };
    }
    pub(crate) use bigint;

    #[macro_export]
    macro_rules! bigint_str {
        ($val: expr) => {
            num_bigint::BigInt::parse_bytes($val.as_bytes(), 10).expect("Couldn't parse bytes")
        };
        ($val: expr, $opt: expr) => {
            num_bigint::BigInt::parse_bytes($val.as_bytes(), $opt).expect("Couldn't parse bytes")
        };
    }
    pub(crate) use bigint_str;

    #[macro_export]
    macro_rules! biguint {
        ($val : expr) => {
            Into::<num_bigint::BigUint>::into($val as u128)
        };
    }
    pub(crate) use biguint;

    #[macro_export]
    macro_rules! biguint_str {
        ($val: expr) => {
            num_bigint::BigUint::parse_bytes($val.as_bytes(), 10).expect("Couldn't parse bytes")
        };
        ($val: expr, $opt: expr) => {
            num_bigint::BigUint::parse_bytes($val.as_bytes(), $opt).expect("Couldn't parse bytes")
        };
    }
    pub(crate) use biguint_str;

    impl From<(&str, u8)> for MaybeRelocatable {
        fn from((string, radix): (&str, u8)) -> Self {
            MaybeRelocatable::Int(felt::felt_str!(string, radix))
        }
    }

    macro_rules! segments {
        ($( (($si:expr, $off:expr), $val:tt) ),* $(,)? ) => {
            {
                let memory = memory!($( (($si, $off), $val) ),*);
                $crate::vm::vm_memory::memory_segments::MemorySegmentManager {
                    memory,
                    segment_sizes: HashMap::new(),
                    segment_used_sizes: None,
                    public_memory_offsets: HashMap::new(),
                }

            }

        };
    }
    pub(crate) use segments;

    macro_rules! memory {
        ( $( (($si:expr, $off:expr), $val:tt) ),* ) => {
            {
                let mut memory = $crate::vm::vm_memory::memory::Memory::new();
                memory_from_memory!(memory, ( $( (($si, $off), $val) ),* ));
                memory
            }
        };
    }
    pub(crate) use memory;

    macro_rules! memory_from_memory {
        ($mem: expr, ( $( (($si:expr, $off:expr), $val:tt) ),* )) => {
            {
                $(
                    memory_inner!($mem, ($si, $off), $val);
                )*
            }
        };
    }
    pub(crate) use memory_from_memory;

    macro_rules! memory_inner {
        ($mem:expr, ($si:expr, $off:expr), ($sival:expr, $offval: expr)) => {
            let (k, v) = (($si, $off).into(), &mayberelocatable!($sival, $offval));
            let mut res = $mem.insert(k, v);
            while matches!(
                res,
                Err($crate::vm::errors::memory_errors::MemoryError::UnallocatedSegment(_))
            ) {
                if $si < 0 {
                    $mem.temp_data.push($crate::stdlib::vec::Vec::new())
                } else {
                    $mem.data.push($crate::stdlib::vec::Vec::new());
                }
                res = $mem.insert(k, v);
            }
        };
        ($mem:expr, ($si:expr, $off:expr), $val:expr) => {
            let (k, v) = (($si, $off).into(), &mayberelocatable!($val));
            let mut res = $mem.insert(k, v);
            while matches!(
                res,
                Err($crate::vm::errors::memory_errors::MemoryError::UnallocatedSegment(_))
            ) {
                if $si < 0 {
                    $mem.temp_data.push($crate::stdlib::vec::Vec::new())
                } else {
                    $mem.data.push($crate::stdlib::vec::Vec::new());
                }
                res = $mem.insert(k, v);
            }
        };
    }
    pub(crate) use memory_inner;

    macro_rules! check_memory {
        ( $mem: expr, $( (($si:expr, $off:expr), $val:tt) ),* $(,)? ) => {
            $(
                check_memory_address!($mem, ($si, $off), $val);
            )*
        };
    }
    pub(crate) use check_memory;

    macro_rules! check_memory_address {
        ($mem:expr, ($si:expr, $off:expr), ($sival:expr, $offval: expr)) => {
            assert_eq!(
                $mem.get(&mayberelocatable!($si, $off)).unwrap().as_ref(),
                &mayberelocatable!($sival, $offval)
            )
        };
        ($mem:expr, ($si:expr, $off:expr), $val:expr) => {
            assert_eq!(
                $mem.get(&mayberelocatable!($si, $off)).unwrap().as_ref(),
                &mayberelocatable!($val)
            )
        };
    }
    pub(crate) use check_memory_address;

    macro_rules! mayberelocatable {
        ($val1 : expr, $val2 : expr) => {
            $crate::types::relocatable::MaybeRelocatable::from(($val1, $val2))
        };
        ($val1 : expr) => {
            $crate::types::relocatable::MaybeRelocatable::from(felt::Felt252::new($val1 as i128))
        };
    }
    pub(crate) use mayberelocatable;

    macro_rules! references {
        ($num: expr) => {{
            let mut references = crate::stdlib::collections::HashMap::<usize, HintReference>::new();
            for i in 0..$num {
                references.insert(i as usize, HintReference::new_simple((i as i32 - $num)));
            }
            references
        }};
    }
    pub(crate) use references;

    macro_rules! vm_with_range_check {
        () => {{
            let mut vm = VirtualMachine::new(false);
            vm.builtin_runners = vec![
                $crate::vm::runners::builtin_runner::RangeCheckBuiltinRunner::new(Some(8), 8, true)
                    .into(),
            ];
            vm
        }};
    }
    pub(crate) use vm_with_range_check;

    macro_rules! cairo_runner {
        ($program:expr) => {
            CairoRunner::new(&$program, "all_cairo", false).unwrap()
        };
        ($program:expr, $layout:expr) => {
            CairoRunner::new(&$program, $layout, false).unwrap()
        };
        ($program:expr, $layout:expr, $proof_mode:expr) => {
            CairoRunner::new(&$program, $layout, $proof_mode).unwrap()
        };
        ($program:expr, $layout:expr, $proof_mode:expr) => {
            CairoRunner::new(&program, $layout.to_string(), proof_mode).unwrap()
        };
    }
    pub(crate) use cairo_runner;

    pub(crate) use crate::stdlib::sync::Arc;
    pub(crate) use crate::types::program::Program;
    pub(crate) use crate::types::program::SharedProgramData;
    macro_rules! program {
        //Empty program
        () => {
            Program::default()
        };
        //Program with builtins
        ( $( $builtin_name: expr ),* ) => {{
            let shared_program_data = SharedProgramData {
                data: crate::stdlib::vec::Vec::new(),
                hints: crate::stdlib::vec::Vec::new(),
                hints_ranges: crate::stdlib::vec::Vec::new(),
                main: None,
                start: None,
                end: None,
                error_message_attributes: crate::stdlib::vec::Vec::new(),
                instruction_locations: None,
                identifiers: crate::stdlib::collections::HashMap::new(),
                reference_manager: Program::get_reference_list(&ReferenceManager {
                    references: crate::stdlib::vec::Vec::new(),
                }),
            };
            Program {
                shared_program_data: Arc::new(shared_program_data),
                constants: crate::stdlib::collections::HashMap::new(),
                builtins: vec![$( $builtin_name ),*],
            }
        }};
        ($($field:ident = $value:expr),* $(,)?) => {{

            let program_flat = crate::utils::test_utils::ProgramFlat {
                $(
                    $field: $value,
                )*
                ..Default::default()
            };

            Into::<Program>::into(program_flat)
        }};
    }

    pub(crate) use program;

    pub(crate) struct ProgramFlat {
        pub(crate) data: crate::utils::Vec<MaybeRelocatable>,
        pub(crate) hints: crate::stdlib::collections::HashMap<
            usize,
            crate::utils::Vec<crate::serde::deserialize_program::HintParams>,
        >,
        pub(crate) main: Option<usize>,
        //start and end labels will only be used in proof-mode
        pub(crate) start: Option<usize>,
        pub(crate) end: Option<usize>,
        pub(crate) error_message_attributes:
            crate::utils::Vec<crate::serde::deserialize_program::Attribute>,
        pub(crate) instruction_locations: Option<
            crate::stdlib::collections::HashMap<
                usize,
                crate::serde::deserialize_program::InstructionLocation,
            >,
        >,
        pub(crate) identifiers: crate::stdlib::collections::HashMap<
            crate::stdlib::string::String,
            crate::serde::deserialize_program::Identifier,
        >,
        pub(crate) constants: crate::stdlib::collections::HashMap<
            crate::stdlib::string::String,
            crate::utils::Felt252,
        >,
        pub(crate) builtins: crate::utils::Vec<crate::serde::deserialize_program::BuiltinName>,
        pub(crate) reference_manager: crate::serde::deserialize_program::ReferenceManager,
    }

    impl Default for ProgramFlat {
        fn default() -> Self {
            Self {
                data: Default::default(),
                hints: Default::default(),
                main: Default::default(),
                start: Default::default(),
                end: Default::default(),
                error_message_attributes: Default::default(),
                instruction_locations: Default::default(),
                identifiers: Default::default(),
                constants: Default::default(),
                builtins: Default::default(),
                reference_manager: crate::serde::deserialize_program::ReferenceManager {
                    references: crate::utils::Vec::new(),
                },
            }
        }
    }

    impl From<ProgramFlat> for Program {
        fn from(val: ProgramFlat) -> Self {
            let (hints, hints_ranges) = Program::flatten_hints(&val.hints);
            Program {
                shared_program_data: Arc::new(SharedProgramData {
                    data: val.data,
                    hints,
                    hints_ranges,
                    main: val.main,
                    start: val.start,
                    end: val.end,
                    error_message_attributes: val.error_message_attributes,
                    instruction_locations: val.instruction_locations,
                    identifiers: val.identifiers,
                    reference_manager: Program::get_reference_list(&val.reference_manager),
                }),
                constants: val.constants,
                builtins: val.builtins,
            }
        }
    }

    macro_rules! vm {
        () => {{
            VirtualMachine::new(false)
        }};

        ($use_trace:expr) => {{
            VirtualMachine::new($use_trace)
        }};
    }
    pub(crate) use vm;

    macro_rules! run_context {
        ( $vm: expr, $pc: expr, $ap: expr, $fp: expr ) => {
            $vm.run_context.pc = Relocatable::from((0, $pc));
            $vm.run_context.ap = $ap;
            $vm.run_context.fp = $fp;
        };
    }
    pub(crate) use run_context;

    macro_rules! ids_data {
        ( $( $name: expr ),* ) => {
            {
                let ids_names = vec![$( $name ),*];
                let references = references!(ids_names.len() as i32);
                let mut ids_data = crate::stdlib::collections::HashMap::<crate::stdlib::string::String, HintReference>::new();
                for (i, name) in ids_names.iter().enumerate() {
                    ids_data.insert(crate::stdlib::string::ToString::to_string(name), references.get(&i).unwrap().clone());
                }
                ids_data
            }
        };
    }
    pub(crate) use ids_data;

    macro_rules! non_continuous_ids_data {
        ( $( ($name: expr, $offset:expr) ),* $(,)? ) => {
            {
                let mut ids_data = crate::stdlib::collections::HashMap::<crate::stdlib::string::String, HintReference>::new();
                $(
                    ids_data.insert(crate::stdlib::string::String::from($name), HintReference::new_simple($offset));
                )*
                ids_data
            }
        };
    }
    pub(crate) use non_continuous_ids_data;

    #[track_caller]
    pub(crate) fn trace_check(actual: &[TraceEntry], expected: &[(usize, usize, usize)]) {
        assert_eq!(actual.len(), expected.len());
        for (entry, expected) in actual.iter().zip(expected.iter()) {
            assert_eq!(&(entry.pc, entry.ap, entry.fp), expected);
        }
    }

    macro_rules! exec_scopes_ref {
        () => {
            &mut ExecutionScopes::new()
        };
    }
    pub(crate) use exec_scopes_ref;

    macro_rules! run_hint {
        ($vm:expr, $ids_data:expr, $hint_code:expr, $exec_scopes:expr, $constants:expr) => {{
            let hint_data = HintProcessorData::new_default($hint_code.to_string(), $ids_data);
            let mut hint_processor = BuiltinHintProcessor::new_empty();
            hint_processor.execute_hint(&mut $vm, $exec_scopes, &any_box!(hint_data), $constants)
        }};
        ($vm:expr, $ids_data:expr, $hint_code:expr, $exec_scopes:expr) => {{
            let hint_data = HintProcessorData::new_default(
                crate::stdlib::string::ToString::to_string($hint_code),
                $ids_data,
            );
            let mut hint_processor = BuiltinHintProcessor::new_empty();
            hint_processor.execute_hint(
                &mut $vm,
                $exec_scopes,
                &any_box!(hint_data),
                &crate::stdlib::collections::HashMap::new(),
            )
        }};
        ($vm:expr, $ids_data:expr, $hint_code:expr) => {{
            let hint_data = HintProcessorData::new_default(
                crate::stdlib::string::ToString::to_string($hint_code),
                $ids_data,
            );
            let mut hint_processor = BuiltinHintProcessor::new_empty();
            hint_processor.execute_hint(
                &mut $vm,
                exec_scopes_ref!(),
                &any_box!(hint_data),
                &crate::stdlib::collections::HashMap::new(),
            )
        }};
    }
    pub(crate) use run_hint;

    macro_rules! add_segments {
        ($vm:expr, $n:expr) => {
            for _ in 0..$n {
                $vm.segments.add();
            }
        };
    }
    pub(crate) use add_segments;

    macro_rules! check_scope {
        ( $exec_scope: expr, [ $( ($name: expr, $val: expr)),*$(,)? ] $(,)? ) => {
            $(
                check_scope_value($exec_scope, $name, $val);
            )*
        };
    }
    pub(crate) use check_scope;

    macro_rules! scope {
        () => { ExecutionScopes::new() };
        (  $( ($name: expr, $val: expr)),* $(,)?  ) => {
            {
                let mut exec_scopes = ExecutionScopes::new();
                $(
                    exec_scopes.assign_or_update_variable(
                        $name,
                        any_box!($val),
                    );
                )*
                exec_scopes
            }
        };
    }
    pub(crate) use scope;

    macro_rules! check_dictionary {
        ( $exec_scopes: expr, $tracker_num:expr, $( ($key:expr, $val:expr )),* ) => {
            $(
                assert_matches::assert_matches!(
                    $exec_scopes
                        .get_dict_manager()
                        .unwrap()
                        .borrow_mut()
                        .trackers
                        .get_mut(&$tracker_num)
                        .unwrap()
                        .get_value(&MaybeRelocatable::from($key)),
                    Ok(x) if x == &MaybeRelocatable::from($val)
                ));
            *
        };
    }
    pub(crate) use check_dictionary;

    macro_rules! check_dict_ptr {
        ($exec_scopes: expr, $tracker_num: expr, ($i:expr, $off:expr)) => {
            assert_eq!(
                $exec_scopes
                    .get_dict_manager()
                    .unwrap()
                    .borrow()
                    .trackers
                    .get(&$tracker_num)
                    .unwrap()
                    .current_ptr,
                relocatable!($i, $off)
            );
        };
    }
    pub(crate) use check_dict_ptr;

    macro_rules! dict_manager {
        ($exec_scopes:expr, $tracker_num:expr, $( ($key:expr, $val:expr )),* ) => {
            let mut tracker = DictTracker::new_empty(relocatable!($tracker_num, 0));
            $(
            tracker.insert_value(&MaybeRelocatable::from($key), &MaybeRelocatable::from($val));
            )*
            let mut dict_manager = DictManager::new();
            dict_manager.trackers.insert(2, tracker);
            $exec_scopes.insert_value("dict_manager", crate::stdlib::rc::Rc::new(core::cell::RefCell::new(dict_manager)))
        };
        ($exec_scopes:expr, $tracker_num:expr) => {
            let  tracker = DictTracker::new_empty(relocatable!($tracker_num, 0));
            let mut dict_manager = DictManager::new();
            dict_manager.trackers.insert(2, tracker);
            $exec_scopes.insert_value("dict_manager", crate::stdlib::rc::Rc::new(core::cell::RefCell::new(dict_manager)))
        };

    }
    pub(crate) use dict_manager;

    macro_rules! dict_manager_default {
        ($exec_scopes:expr, $tracker_num:expr,$default:expr, $( ($key:expr, $val:expr )),* ) => {
            let mut tracker = DictTracker::new_default_dict(relocatable!($tracker_num, 0), &MaybeRelocatable::from($default), None);
            $(
            tracker.insert_value(&MaybeRelocatable::from($key), &MaybeRelocatable::from($val));
            )*
            let mut dict_manager = DictManager::new();
            dict_manager.trackers.insert(2, tracker);
            $exec_scopes.insert_value("dict_manager", crate::stdlib::rc::Rc::new(core::cell::RefCell::new(dict_manager)))
        };
        ($exec_scopes:expr, $tracker_num:expr,$default:expr) => {
            let tracker = DictTracker::new_default_dict(relocatable!($tracker_num, 0), &MaybeRelocatable::from($default), None);
            let mut dict_manager = DictManager::new();
            dict_manager.trackers.insert(2, tracker);
            $exec_scopes.insert_value("dict_manager", crate::stdlib::rc::Rc::new(core::cell::RefCell::new(dict_manager)))
        };
    }
    pub(crate) use dict_manager_default;

    macro_rules! vec_data {
        ( $( ($val:tt) ),* ) => {
            vec![$( vec_data_inner!($val) ),*]
        };
    }
    pub(crate) use vec_data;

    macro_rules! vec_data_inner {
        (( $val1:expr, $val2:expr )) => {
            mayberelocatable!($val1, $val2)
        };
        ( $val:expr ) => {
            mayberelocatable!($val)
        };
    }
    pub(crate) use vec_data_inner;

    pub fn check_scope_value<T: core::fmt::Debug + core::cmp::PartialEq + 'static>(
        scopes: &ExecutionScopes,
        name: &str,
        value: T,
    ) {
        let scope_value = scopes.get_any_boxed_ref(name).unwrap();
        assert_eq!(scope_value.downcast_ref::<T>(), Some(&value));
    }
}

#[cfg(test)]
mod test {
    use crate::hint_processor::hint_processor_definition::HintProcessorLogic;
    use crate::stdlib::{cell::RefCell, collections::HashMap, rc::Rc, string::String, vec::Vec};
    use crate::{
        hint_processor::{
            builtin_hint_processor::{
                builtin_hint_processor_definition::{BuiltinHintProcessor, HintProcessorData},
                dict_manager::{DictManager, DictTracker},
            },
            hint_processor_definition::HintReference,
        },
        serde::deserialize_program::{BuiltinName, ReferenceManager},
        types::{exec_scope::ExecutionScopes, program::Program, relocatable::MaybeRelocatable},
        utils::test_utils::*,
        vm::{trace::trace_entry::TraceEntry, vm_core::VirtualMachine, vm_memory::memory::Memory},
    };
    use felt::Felt252;
    use num_traits::One;

    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::*;

    use super::*;

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn memory_macro_test() {
        let mut memory = Memory::new();
        for _ in 0..2 {
            memory.data.push(Vec::new());
        }
        memory
            .insert(
                Relocatable::from((1, 2)),
                &MaybeRelocatable::from(Felt252::one()),
            )
            .unwrap();
        memory
            .insert(Relocatable::from((1, 1)), &MaybeRelocatable::from((1, 0)))
            .unwrap();
        let mem = memory![((1, 2), 1), ((1, 1), (1, 0))];
        assert_eq!(memory.data, mem.data);
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn check_memory_macro_test() {
        let mut memory = Memory::new();
        for _ in 0..2 {
            memory.data.push(Vec::new());
        }
        memory
            .insert(Relocatable::from((1, 1)), &MaybeRelocatable::from((1, 0)))
            .unwrap();

        memory
            .insert(
                Relocatable::from((1, 2)),
                &MaybeRelocatable::from(Felt252::one()),
            )
            .unwrap();

        check_memory![memory, ((1, 1), (1, 0)), ((1, 2), 1)];
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn check_memory_address_macro_test() {
        let mut memory = Memory::new();
        for _ in 0..2 {
            memory.data.push(Vec::new());
        }
        memory
            .insert(Relocatable::from((1, 1)), &MaybeRelocatable::from((1, 0)))
            .unwrap();

        memory
            .insert(
                Relocatable::from((1, 2)),
                &MaybeRelocatable::from(Felt252::one()),
            )
            .unwrap();

        check_memory_address!(memory, (1, 1), (1, 0));
        check_memory_address!(memory, (1, 2), 1);
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn create_run_context() {
        let mut vm = vm!();
        run_context!(vm, 2, 6, 10);

        assert_eq!(vm.run_context.pc, Relocatable::from((0, 2)));
        assert_eq!(vm.run_context.ap, 6);
        assert_eq!(vm.run_context.fp, 10);
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn assert_trace() {
        let trace = vec![
            TraceEntry {
                pc: 2,
                ap: 7,
                fp: 1,
            },
            TraceEntry {
                pc: 5,
                ap: 1,
                fp: 0,
            },
            TraceEntry {
                pc: 9,
                ap: 5,
                fp: 7,
            },
        ];
        trace_check(&trace, &[(2, 7, 1), (5, 1, 0), (9, 5, 7)]);
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn test_non_continuous_ids_data() {
        let ids_data_macro = non_continuous_ids_data![("a", -2), ("", -6)];
        let ids_data_verbose = HashMap::from([
            ("a".to_string(), HintReference::new_simple(-2)),
            ("".to_string(), HintReference::new_simple(-6)),
        ]);
        assert_eq!(ids_data_macro, ids_data_verbose);
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_hint_alloc() {
        let hint_code = "memory[ap] = segments.add()";
        let mut vm = vm!();
        add_segments!(vm, 1);
        assert_matches::assert_matches!(run_hint!(vm, HashMap::new(), hint_code), Ok(()));
        //A segment is added
        assert_eq!(vm.segments.memory.data.len(), 2);
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn check_scope_test_pass() {
        let mut exec_scopes = ExecutionScopes::new();
        exec_scopes.assign_or_update_variable("a", any_box!(String::from("Hello")));
        exec_scopes.assign_or_update_variable(
            "",
            any_box!(Rc::new(RefCell::new(HashMap::<usize, Vec<usize>>::new()))),
        );
        exec_scopes.assign_or_update_variable("c", any_box!(vec![1, 2, 3, 4]));
        check_scope!(
            &exec_scopes,
            [
                ("a", String::from("Hello")),
                (
                    "",
                    Rc::new(RefCell::new(HashMap::<usize, Vec<usize>>::new()))
                ),
                ("c", vec![1, 2, 3, 4])
            ]
        );
    }

    #[test]
    #[should_panic]
    fn check_scope_test_fail() {
        let mut exec_scopes = ExecutionScopes::new();
        exec_scopes.assign_or_update_variable("a", any_box!(String::from("Hello")));
        exec_scopes.assign_or_update_variable(
            "",
            any_box!(Rc::new(RefCell::new(HashMap::<usize, Vec<usize>>::new()))),
        );
        exec_scopes.assign_or_update_variable("c", any_box!(vec![1, 2, 3, 4]));
        check_scope!(
            &exec_scopes,
            [
                ("a", String::from("Hello")),
                (
                    "",
                    Rc::new(RefCell::new(HashMap::<usize, Vec<usize>>::new()))
                ),
                ("c", vec![1, 2, 3, 5])
            ]
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn scope_macro_test() {
        let scope_from_macro = scope![("a", Felt252::one())];
        let mut scope_verbose = ExecutionScopes::new();
        scope_verbose.assign_or_update_variable("a", any_box!(Felt252::one()));
        assert_eq!(scope_from_macro.data.len(), scope_verbose.data.len());
        assert_eq!(scope_from_macro.data[0].len(), scope_verbose.data[0].len());
        assert_eq!(
            scope_from_macro.data[0].get("a").unwrap().downcast_ref(),
            Some(&Felt252::one())
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn check_dictionary_pass() {
        let mut tracker = DictTracker::new_empty(relocatable!(2, 0));
        tracker.insert_value(
            &MaybeRelocatable::from(Felt252::new(5)),
            &MaybeRelocatable::from(Felt252::new(10)),
        );
        let mut dict_manager = DictManager::new();
        dict_manager.trackers.insert(2, tracker);
        let mut exec_scopes = ExecutionScopes::new();
        exec_scopes.assign_or_update_variable(
            "dict_manager",
            any_box!(Rc::new(RefCell::new(dict_manager))),
        );
        check_dictionary!(&exec_scopes, 2, (5, 10));
    }

    #[test]
    #[should_panic]
    fn check_dictionary_fail() {
        let mut tracker = DictTracker::new_empty(relocatable!(2, 0));
        tracker.insert_value(
            &MaybeRelocatable::from(Felt252::new(5)),
            &MaybeRelocatable::from(Felt252::new(10)),
        );
        let mut dict_manager = DictManager::new();
        dict_manager.trackers.insert(2, tracker);
        let mut exec_scopes = ExecutionScopes::new();
        exec_scopes.assign_or_update_variable(
            "dict_manager",
            any_box!(Rc::new(RefCell::new(dict_manager))),
        );
        check_dictionary!(&exec_scopes, 2, (5, 11));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn check_dict_ptr_pass() {
        let tracker = DictTracker::new_empty(relocatable!(2, 0));
        let mut dict_manager = DictManager::new();
        dict_manager.trackers.insert(2, tracker);
        let mut exec_scopes = ExecutionScopes::new();
        exec_scopes.assign_or_update_variable(
            "dict_manager",
            any_box!(Rc::new(RefCell::new(dict_manager))),
        );
        check_dict_ptr!(&exec_scopes, 2, (2, 0));
    }

    #[test]
    #[should_panic]
    fn check_dict_ptr_fail() {
        let tracker = DictTracker::new_empty(relocatable!(2, 0));
        let mut dict_manager = DictManager::new();
        dict_manager.trackers.insert(2, tracker);
        let mut exec_scopes = ExecutionScopes::new();
        exec_scopes.assign_or_update_variable(
            "dict_manager",
            any_box!(Rc::new(RefCell::new(dict_manager))),
        );
        check_dict_ptr!(&exec_scopes, 2, (3, 0));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn dict_manager_macro() {
        let tracker = DictTracker::new_empty(relocatable!(2, 0));
        let mut dict_manager = DictManager::new();
        dict_manager.trackers.insert(2, tracker);
        let mut exec_scopes = ExecutionScopes::new();
        dict_manager!(exec_scopes, 2);
        assert_matches::assert_matches!(
            exec_scopes.get_dict_manager(),
            Ok(x) if x == Rc::new(RefCell::new(dict_manager))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn dict_manager_default_macro() {
        let tracker = DictTracker::new_default_dict(
            relocatable!(2, 0),
            &MaybeRelocatable::from(Felt252::new(17)),
            None,
        );
        let mut dict_manager = DictManager::new();
        dict_manager.trackers.insert(2, tracker);
        let mut exec_scopes = ExecutionScopes::new();
        dict_manager_default!(exec_scopes, 2, 17);
        assert_matches::assert_matches!(
            exec_scopes.get_dict_manager(),
            Ok(x) if x == Rc::new(RefCell::new(dict_manager))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn data_vec_test() {
        let data = vec_data!((1), ((2, 2)), (("49128305", 10)), (("3b6f00a9", 16)));
        assert_eq!(data[0], mayberelocatable!(1));
        assert_eq!(data[1], mayberelocatable!(2, 2));
        assert_eq!(data[2], mayberelocatable!(49128305));
        assert_eq!(data[3], mayberelocatable!(997130409));
    }
    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn from_relocatable_to_indexes_test() {
        let reloc_1 = relocatable!(1, 5);
        let reloc_2 = relocatable!(0, 5);
        let reloc_3 = relocatable!(-1, 5);
        assert_eq!((1, 5), from_relocatable_to_indexes(reloc_1));
        assert_eq!((0, 5), from_relocatable_to_indexes(reloc_2));
        assert_eq!((0, 5), from_relocatable_to_indexes(reloc_3));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn program_macro() {
        let shared_data = SharedProgramData {
            data: Vec::new(),
            hints: Vec::new(),
            hints_ranges: Vec::new(),
            main: None,
            start: None,
            end: None,
            error_message_attributes: Vec::new(),
            instruction_locations: None,
            identifiers: HashMap::new(),
            reference_manager: Program::get_reference_list(&ReferenceManager {
                references: Vec::new(),
            }),
        };
        let program = Program {
            shared_program_data: Arc::new(shared_data),
            constants: HashMap::new(),
            builtins: Vec::new(),
        };
        assert_eq!(program, program!())
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn program_macro_with_builtin() {
        let shared_data = SharedProgramData {
            data: Vec::new(),
            hints: Vec::new(),
            hints_ranges: Vec::new(),
            main: None,
            start: None,
            end: None,
            error_message_attributes: Vec::new(),
            instruction_locations: None,
            identifiers: HashMap::new(),
            reference_manager: Program::get_reference_list(&ReferenceManager {
                references: Vec::new(),
            }),
        };
        let program = Program {
            shared_program_data: Arc::new(shared_data),
            constants: HashMap::new(),
            builtins: vec![BuiltinName::range_check],
        };

        assert_eq!(program, program![BuiltinName::range_check])
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn program_macro_custom_definition() {
        let shared_data = SharedProgramData {
            data: Vec::new(),
            hints: Vec::new(),
            hints_ranges: Vec::new(),
            main: Some(2),
            start: None,
            end: None,
            error_message_attributes: Vec::new(),
            instruction_locations: None,
            identifiers: HashMap::new(),
            reference_manager: Program::get_reference_list(&ReferenceManager {
                references: Vec::new(),
            }),
        };
        let program = Program {
            shared_program_data: Arc::new(shared_data),
            constants: HashMap::new(),
            builtins: vec![BuiltinName::range_check],
        };

        assert_eq!(
            program,
            program!(builtins = vec![BuiltinName::range_check], main = Some(2),)
        )
    }
}
