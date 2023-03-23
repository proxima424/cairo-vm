use core::str::FromStr;

use super::bigint_utils::BigInt3;
use crate::{
    hint_processor::hint_processor_utils::{
        compute_addr_from_reference, get_integer_from_reference,
    },
    stdlib::{collections::HashMap, ops::BitAnd, prelude::*},
};
use crate::{
    hint_processor::{
        builtin_hint_processor::{hint_utils::insert_value_into_ap, secp::secp_utils::pack},
        hint_processor_definition::HintReference,
    },
    math_utils::{ec_double_slope, line_slope},
    serde::deserialize_program::ApTracking,
    types::exec_scope::ExecutionScopes,
    vm::{errors::hint_errors::HintError, vm_core::VirtualMachine},
};
use felt::Felt252;
use lazy_static::lazy_static;
use num_bigint::BigInt;
use num_integer::Integer;
use num_traits::{One, Zero};

lazy_static! {
    static ref SECP_P: BigInt = BigInt::from_str(
        "115792089237316195423570985008687907853269984665640564039457584007908834671663"
    )
    .unwrap();
}

#[derive(Debug, PartialEq)]
struct EcPoint<'a> {
    x: BigInt3<'a>,
    y: BigInt3<'a>,
}
impl EcPoint<'_> {
    fn from_reference<'a>(
        name: &'a str,
        vm: &'a VirtualMachine,
        reference: &HintReference,
        ap_tracking: &ApTracking,
    ) -> Result<EcPoint<'a>, HintError> {
        // Get first addr of EcPoint struct
        let point_addr = compute_addr_from_reference(reference, vm, ap_tracking)
            .ok_or(HintError::UnknownIdentifier(name.to_string()))?;
        Ok(EcPoint {
            x: BigInt3::from_base_addr(point_addr, &format!("{}.x", name), vm)?,
            y: BigInt3::from_base_addr((point_addr + 3)?, &format!("{}.y", name), vm)?,
        })
    }
}

/*
Implements hint:
%{
    from starkware.cairo.common.cairo_secp.secp_utils import SECP_P, pack

    y = pack(ids.point.y, PRIME) % SECP_P
    # The modulo operation in python always returns a nonnegative number.
    value = (-y) % SECP_P
%}
*/
#[derive(Debug)]
pub struct EcNegateHintData {
    point: HintReference,
    ap_tracking: ApTracking,
}

impl EcNegateHintData {
    pub fn compile(
        reference_ids: &HashMap<String, usize>,
        references: &HashMap<usize, HintReference>,
        ap_tracking: &ApTracking,
    ) -> Option<Self> {
        Some(EcNegateHintData {
            point: references
                .get(reference_ids.get("starkware.cairo.common.cairo_secp.ec.ec_negate.point")?)?
                .clone(),
            ap_tracking: ap_tracking.clone(),
        })
    }

    pub fn execute(
        &self,
        vm: &VirtualMachine,
        exec_scopes: &mut ExecutionScopes,
    ) -> Result<(), HintError> {
        //ids.point
        let point_y = (compute_addr_from_reference(&self.point, vm, &self.ap_tracking)
            .ok_or(HintError::UnknownIdentifier("point".to_string()))?
            + 3i32)?;
        let y_bigint3 = BigInt3::from_base_addr(point_y, "point.y", vm)?;
        let y = pack(y_bigint3);
        let value = (-y).mod_floor(&SECP_P);
        exec_scopes.insert_value("value", value);
        Ok(())
    }
}

/*
Implements hint:
%{
    from starkware.cairo.common.cairo_secp.secp_utils import SECP_P, pack
    from starkware.python.math_utils import ec_double_slope

    # Compute the slope.
    x = pack(ids.point.x, PRIME)
    y = pack(ids.point.y, PRIME)
    value = slope = ec_double_slope(point=(x, y), alpha=0, p=SECP_P)
%}
*/
#[derive(Debug)]
pub struct ComputeDoublingSlopeData {
    point: HintReference,
    ap_tracking: ApTracking,
}

impl ComputeDoublingSlopeData {
    pub fn compile(
        reference_ids: &HashMap<String, usize>,
        references: &HashMap<usize, HintReference>,
        ap_tracking: &ApTracking,
    ) -> Option<Self> {
        Some(ComputeDoublingSlopeData {
            point: references
                .get(
                    reference_ids
                        .get("starkware.cairo.common.cairo_secp.ec.compute_doubling_slope.point")?,
                )?
                .clone(),
            ap_tracking: ap_tracking.clone(),
        })
    }
    pub fn execute(
        &self,
        vm: &mut VirtualMachine,
        exec_scopes: &mut ExecutionScopes,
    ) -> Result<(), HintError> {
        //ids.point
        let point = EcPoint::from_reference("point", vm, &self.point, &self.ap_tracking)?;

        let value = ec_double_slope(&(pack(point.x), pack(point.y)), &BigInt::zero(), &SECP_P);
        exec_scopes.insert_value("value", value.clone());
        exec_scopes.insert_value("slope", value);
        Ok(())
    }
}

/*
Implements hint:
%{
    from starkware.cairo.common.cairo_secp.secp_utils import SECP_P, pack
    from starkware.python.math_utils import line_slope

    # Compute the slope.
    x0 = pack(ids.point0.x, PRIME)
    y0 = pack(ids.point0.y, PRIME)
    x1 = pack(ids.point1.x, PRIME)
    y1 = pack(ids.point1.y, PRIME)
    value = slope = line_slope(point1=(x0, y0), point2=(x1, y1), p=SECP_P)
%}
*/
#[derive(Debug)]
pub struct ComputeSlopeData {
    point0: HintReference,
    point1: HintReference,
    ap_tracking: ApTracking,
}

impl ComputeSlopeData {
    pub fn compile(
        reference_ids: &HashMap<String, usize>,
        references: &HashMap<usize, HintReference>,
        ap_tracking: &ApTracking,
    ) -> Option<Self> {
        Some(ComputeSlopeData {
            point0: references
                .get(
                    reference_ids
                        .get("starkware.cairo.common.cairo_secp.ec.compute_slope.point0")?,
                )?
                .clone(),
            point1: references
                .get(
                    reference_ids
                        .get("starkware.cairo.common.cairo_secp.ec.compute_slope.point1")?,
                )?
                .clone(),
            ap_tracking: ap_tracking.clone(),
        })
    }

    pub fn execute(
        &self,
        vm: &VirtualMachine,
        exec_scopes: &mut ExecutionScopes,
    ) -> Result<(), HintError> {
        //ids.point0
        let point0 = EcPoint::from_reference("point0", vm, &self.point0, &self.ap_tracking)?;
        //ids.point1
        let point1 = EcPoint::from_reference("point1", vm, &self.point1, &self.ap_tracking)?;

        let value = line_slope(
            &(pack(point0.x), pack(point0.y)),
            &(pack(point1.x), pack(point1.y)),
            &SECP_P,
        );
        exec_scopes.insert_value("value", value.clone());
        exec_scopes.insert_value("slope", value);
        Ok(())
    }
}

/*
Implements hint:
%{
    from starkware.cairo.common.cairo_secp.secp_utils import SECP_P, pack

    slope = pack(ids.slope, PRIME)
    x = pack(ids.point.x, PRIME)
    y = pack(ids.point.y, PRIME)

    value = new_x = (pow(slope, 2, SECP_P) - 2 * x) % SECP_P
%}
*/

#[derive(Debug)]
pub struct EcDoubleAssignNewXData {
    slope: HintReference,
    point: HintReference,
    ap_tracking: ApTracking,
}

impl EcDoubleAssignNewXData {
    pub fn compile(
        reference_ids: &HashMap<String, usize>,
        references: &HashMap<usize, HintReference>,
        ap_tracking: &ApTracking,
    ) -> Option<Self> {
        Some(EcDoubleAssignNewXData {
            slope: references
                .get(reference_ids.get("starkware.cairo.common.cairo_secp.ec.ec_double.slope")?)?
                .clone(),
            point: references
                .get(reference_ids.get("starkware.cairo.common.cairo_secp.ec.ec_double.point")?)?
                .clone(),
            ap_tracking: ap_tracking.clone(),
        })
    }

    pub fn execute(
        &self,
        vm: &VirtualMachine,
        exec_scopes: &mut ExecutionScopes,
    ) -> Result<(), HintError> {
        //ids.slope
        let slope = BigInt3::from_base_addr(
            compute_addr_from_reference(&self.slope, vm, &self.ap_tracking)
                .ok_or(HintError::UnknownIdentifier("slope".to_string()))?,
            "slope",
            vm,
        )?;
        //ids.point
        let point = EcPoint::from_reference("point", vm, &self.point, &self.ap_tracking)?;

        let slope = pack(slope);
        let x = pack(point.x);
        let y = pack(point.y);

        let value = (slope.pow(2) - (&x << 1u32)).mod_floor(&SECP_P);

        //Assign variables to vm scope
        exec_scopes.insert_value("slope", slope);
        exec_scopes.insert_value("x", x);
        exec_scopes.insert_value("y", y);
        exec_scopes.insert_value("value", value.clone());
        exec_scopes.insert_value("new_x", value);
        Ok(())
    }
}

/*
Implements hint:
%{ value = new_y = (slope * (x - new_x) - y) % SECP_P %}
*/
#[derive(Debug)]
pub struct EcDoubleAssignNewYData {}

impl EcDoubleAssignNewYData {
    pub fn compile(
        _reference_ids: &HashMap<String, usize>,
        _references: &HashMap<usize, HintReference>,
        _ap_tracking: &ApTracking,
    ) -> Option<Self> {
        Some(EcDoubleAssignNewYData {})
    }

    pub fn execute(
        &self,
        _vm: &VirtualMachine,
        exec_scopes: &mut ExecutionScopes,
    ) -> Result<(), HintError> {
        //Get variables from vm scope
        let (slope, x, new_x, y) = (
            exec_scopes.get::<BigInt>("slope")?,
            exec_scopes.get::<BigInt>("x")?,
            exec_scopes.get::<BigInt>("new_x")?,
            exec_scopes.get::<BigInt>("y")?,
        );

        let value = (slope * (x - new_x) - y).mod_floor(&SECP_P);
        exec_scopes.insert_value("value", value.clone());
        exec_scopes.insert_value("new_y", value);
        Ok(())
    }
}

/*
Implements hint:
%{
    from starkware.cairo.common.cairo_secp.secp_utils import SECP_P, pack

    slope = pack(ids.slope, PRIME)
    x0 = pack(ids.point0.x, PRIME)
    x1 = pack(ids.point1.x, PRIME)
    y0 = pack(ids.point0.y, PRIME)

    value = new_x = (pow(slope, 2, SECP_P) - x0 - x1) % SECP_P
%}
*/

#[derive(Debug)]
pub struct FastEcAddAssignXData {
    point0: HintReference,
    point1: HintReference,
    slope: HintReference,
    ap_tracking: ApTracking,
}

impl FastEcAddAssignXData {
    pub fn compile(
        reference_ids: &HashMap<String, usize>,
        references: &HashMap<usize, HintReference>,
        ap_tracking: &ApTracking,
    ) -> Option<Self> {
        Some(FastEcAddAssignXData {
            point0: references
                .get(reference_ids.get("starkware.cairo.common.cairo_secp.ec.fast_ec_add.point0")?)?
                .clone(),
            point1: references
                .get(reference_ids.get("starkware.cairo.common.cairo_secp.ec.fast_ec_add.point1")?)?
                .clone(),
            slope: references
                .get(reference_ids.get("starkware.cairo.common.cairo_secp.ec.fast_ec_add.slope")?)?
                .clone(),
            ap_tracking: ap_tracking.clone(),
        })
    }

    pub fn execute(
        &self,
        vm: &VirtualMachine,
        exec_scopes: &mut ExecutionScopes,
    ) -> Result<(), HintError> {
        //ids.point0
        let point0 = EcPoint::from_reference("point0", vm, &self.point0, &self.ap_tracking)?;
        //ids.point1
        let point1 = EcPoint::from_reference("point1", vm, &self.point1, &self.ap_tracking)?;
        //ids.slope
        let slope = BigInt3::from_base_addr(
            compute_addr_from_reference(&self.slope, vm, &self.ap_tracking)
                .ok_or(HintError::UnknownIdentifier("slope".to_string()))?,
            "slope",
            vm,
        )?;

        let slope = pack(slope);
        let x0 = pack(point0.x);
        let x1 = pack(point1.x);
        let y0 = pack(point0.y);

        let value = (&slope * &slope - &x0 - &x1).mod_floor(&SECP_P);
        //Assign variables to vm scope
        exec_scopes.insert_value("slope", slope);
        exec_scopes.insert_value("x0", x0);
        exec_scopes.insert_value("y0", y0);
        exec_scopes.insert_value("value", value.clone());
        exec_scopes.insert_value("new_x", value);

        Ok(())
    }
}

/*
Implements hint:
%{ value = new_y = (slope * (x0 - new_x) - y0) % SECP_P %}
*/
#[derive(Debug)]
pub struct FastEcAddAssignYData {}

impl FastEcAddAssignYData {
    pub fn compile(
        _reference_ids: &HashMap<String, usize>,
        _references: &HashMap<usize, HintReference>,
        _ap_tracking: &ApTracking,
    ) -> Option<Self> {
        Some(FastEcAddAssignYData {})
    }

    pub fn execute(
        &self,
        _vm: &VirtualMachine,
        exec_scopes: &mut ExecutionScopes,
    ) -> Result<(), HintError> {
        //Get variables from vm scope
        let (slope, x0, new_x, y0) = (
            exec_scopes.get::<BigInt>("slope")?,
            exec_scopes.get::<BigInt>("x0")?,
            exec_scopes.get::<BigInt>("new_x")?,
            exec_scopes.get::<BigInt>("y0")?,
        );
        let value = (slope * (x0 - new_x) - y0).mod_floor(&SECP_P);
        exec_scopes.insert_value("value", value.clone());
        exec_scopes.insert_value("new_y", value);

        Ok(())
    }
}

/*
Implements hint:
%{ memory[ap] = (ids.scalar % PRIME) % 2 %}
*/
#[derive(Debug)]
pub struct EcMulInnerData {
    scalar: HintReference,
    ap_tracking: ApTracking,
}

impl EcMulInnerData {
    pub fn compile(
        reference_ids: &HashMap<String, usize>,
        references: &HashMap<usize, HintReference>,
        ap_tracking: &ApTracking,
    ) -> Option<Self> {
        Some(EcMulInnerData {
            scalar: references
                .get(
                    reference_ids
                        .get("starkware.cairo.common.cairo_secp.ec.ec_mul_inner.scalar")?,
                )?
                .clone(),
            ap_tracking: ap_tracking.clone(),
        })
    }

    pub fn execute(
        &self,
        vm: &mut VirtualMachine,
        _exec_scopes: &mut ExecutionScopes,
    ) -> Result<(), HintError> {
        //(ids.scalar % PRIME) % 2
        let scalar = get_integer_from_reference(vm, &self.scalar, &self.ap_tracking)
            .map_err(|_| HintError::UnknownIdentifier("scalar".to_string()))?
            .as_ref()
            .bitand(&Felt252::one());
        insert_value_into_ap(vm, scalar)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stdlib::string::ToString;
    use crate::vm::vm_memory::memory_segments::MemorySegmentManager;
    use crate::{
        any_box,
        hint_processor::{
            builtin_hint_processor::builtin_hint_processor_definition::{
                BuiltinHintProcessor, HintProcessorData,
            },
            hint_processor_definition::HintProcessor,
        },
        types::{
            exec_scope::ExecutionScopes,
            relocatable::{MaybeRelocatable, Relocatable},
        },
        utils::test_utils::*,
        vm::{
            errors::memory_errors::MemoryError, runners::builtin_runner::RangeCheckBuiltinRunner,
            vm_core::VirtualMachine, vm_memory::memory::Memory,
        },
    };
    use assert_matches::assert_matches;

    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::*;

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_ec_negate_ok() {
        let hint_code = "from starkware.cairo.common.cairo_secp.secp_utils import SECP_P, pack\n\ny = pack(ids.point.y, PRIME) % SECP_P\n# The modulo operation in python always returns a nonnegative number.\nvalue = (-y) % SECP_P";
        let mut vm = vm_with_range_check!();

        vm.segments = segments![((1, 3), 2645i32), ((1, 4), 454i32), ((1, 5), 206i32)];
        //Initialize fp
        vm.run_context.fp = 1;
        //Create hint_data
        let ids_data = ids_data!["point"];
        let mut exec_scopes = ExecutionScopes::new();
        //Execute the hint
        assert_matches!(
            run_hint!(
                vm,
                ids_data,
                hint_code,
                &mut exec_scopes,
                &[(
                    SECP_REM,
                    Felt252::one().shl(32_u32)
                        + Felt252::one().shl(9_u32)
                        + Felt252::one().shl(8_u32)
                        + Felt252::one().shl(7_u32)
                        + Felt252::one().shl(6_u32)
                        + Felt252::one().shl(4_u32)
                        + Felt252::one()
                )]
                .into_iter()
                .map(|(k, v)| (k.to_string(), v))
                .collect()
            ),
            Ok(())
        );
        //Check 'value' is defined in the vm scope
        assert_matches!(
            exec_scopes.get::<BigInt>("value"),
            Ok(x) if x == bigint_str!(
                "115792089237316195423569751828682367333329274433232027476421668138471189901786"
            )
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_compute_doubling_slope_ok() {
        let hint_code = "from starkware.cairo.common.cairo_secp.secp_utils import SECP_P, pack\nfrom starkware.python.math_utils import ec_double_slope\n\n# Compute the slope.\nx = pack(ids.point.x, PRIME)\ny = pack(ids.point.y, PRIME)\nvalue = slope = ec_double_slope(point=(x, y), alpha=0, p=SECP_P)";
        let mut vm = vm_with_range_check!();
        vm.segments = segments![
            ((1, 0), 614323u64),
            ((1, 1), 5456867u64),
            ((1, 2), 101208u64),
            ((1, 3), 773712524u64),
            ((1, 4), 77371252u64),
            ((1, 5), 5298795u64)
        ];

        //Initialize fp
        vm.run_context.fp = 1;

        let ids_data = ids_data!["point"];
        let mut exec_scopes = ExecutionScopes::new();

        //Execute the hint
        assert_matches!(
            run_hint!(
                vm,
                ids_data,
                hint_code,
                &mut exec_scopes,
                &[(
                    SECP_REM,
                    Felt252::one().shl(32_u32)
                        + Felt252::one().shl(9_u32)
                        + Felt252::one().shl(8_u32)
                        + Felt252::one().shl(7_u32)
                        + Felt252::one().shl(6_u32)
                        + Felt252::one().shl(4_u32)
                        + Felt252::one()
                )]
                .into_iter()
                .map(|(k, v)| (k.to_string(), v))
                .collect()
            ),
            Ok(())
        );
        check_scope!(
            &exec_scopes,
            [
                (
                    "value",
                    bigint_str!(
            "40442433062102151071094722250325492738932110061897694430475034100717288403728"
        )
                ),
                (
                    "slope",
                    bigint_str!(
            "40442433062102151071094722250325492738932110061897694430475034100717288403728"
        )
                )
            ]
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_compute_slope_ok() {
        let hint_code = "from starkware.cairo.common.cairo_secp.secp_utils import SECP_P, pack\nfrom starkware.python.math_utils import line_slope\n\n# Compute the slope.\nx0 = pack(ids.point0.x, PRIME)\ny0 = pack(ids.point0.y, PRIME)\nx1 = pack(ids.point1.x, PRIME)\ny1 = pack(ids.point1.y, PRIME)\nvalue = slope = line_slope(point1=(x0, y0), point2=(x1, y1), p=SECP_P)";
        let mut vm = vm_with_range_check!();

        //Insert ids.point0 and ids.point1 into memory
        vm.segments = segments![
            ((1, 0), 134),
            ((1, 1), 5123),
            ((1, 2), 140),
            ((1, 3), 1232),
            ((1, 4), 4652),
            ((1, 5), 720),
            ((1, 6), 156),
            ((1, 7), 6545),
            ((1, 8), 100010),
            ((1, 9), 1123),
            ((1, 10), 1325),
            ((1, 11), 910)
        ];

        //Initialize fp
        vm.run_context.fp = 14;
        let ids_data = HashMap::from([
            ("point0".to_string(), HintReference::new_simple(-14)),
            ("point1".to_string(), HintReference::new_simple(-8)),
        ]);
        let mut exec_scopes = ExecutionScopes::new();

        //Execute the hint
        assert_matches!(
            run_hint!(
                vm,
                ids_data,
                hint_code,
                &mut exec_scopes,
                &[(
                    SECP_REM,
                    Felt252::one().shl(32_u32)
                        + Felt252::one().shl(9_u32)
                        + Felt252::one().shl(8_u32)
                        + Felt252::one().shl(7_u32)
                        + Felt252::one().shl(6_u32)
                        + Felt252::one().shl(4_u32)
                        + Felt252::one()
                )]
                .into_iter()
                .map(|(k, v)| (k.to_string(), v))
                .collect()
            ),
            Ok(())
        );
        check_scope!(
            &exec_scopes,
            [
                (
                    "value",
                    bigint_str!(
            "41419765295989780131385135514529906223027172305400087935755859001910844026631"
        )
                ),
                (
                    "slope",
                    bigint_str!(
            "41419765295989780131385135514529906223027172305400087935755859001910844026631"
        )
                )
            ]
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_ec_double_assign_new_x_ok() {
        let hint_code = "from starkware.cairo.common.cairo_secp.secp_utils import SECP_P, pack\n\nslope = pack(ids.slope, PRIME)\nx = pack(ids.point.x, PRIME)\ny = pack(ids.point.y, PRIME)\n\nvalue = new_x = (pow(slope, 2, SECP_P) - 2 * x) % SECP_P";
        let mut vm = vm_with_range_check!();

        //Insert ids.point and ids.slope into memory
        vm.segments = segments![
            ((1, 0), 134),
            ((1, 1), 5123),
            ((1, 2), 140),
            ((1, 3), 1232),
            ((1, 4), 4652),
            ((1, 5), 720),
            ((1, 6), 44186171158942157784255469_i128),
            ((1, 7), 54173758974262696047492534_i128),
            ((1, 8), 8106299688661572814170174_i128)
        ];

        //Initialize fp
        vm.run_context.fp = 10;
        let ids_data = HashMap::from([
            ("point".to_string(), HintReference::new_simple(-10)),
            ("slope".to_string(), HintReference::new_simple(-4)),
        ]);
        let mut exec_scopes = ExecutionScopes::new();

        //Execute the hint
        assert_matches!(
            run_hint!(
                vm,
                ids_data,
                hint_code,
                &mut exec_scopes,
                &[(
                    SECP_REM,
                    Felt252::one().shl(32_u32)
                        + Felt252::one().shl(9_u32)
                        + Felt252::one().shl(8_u32)
                        + Felt252::one().shl(7_u32)
                        + Felt252::one().shl(6_u32)
                        + Felt252::one().shl(4_u32)
                        + Felt252::one()
                )]
                .into_iter()
                .map(|(k, v)| (k.to_string(), v))
                .collect()
            ),
            Ok(())
        );

        check_scope!(
            &exec_scopes,
            [
                (
                    "slope",
                    bigint_str!(
            "48526828616392201132917323266456307435009781900148206102108934970258721901549"
        )
                ),
                (
                    "x",
                    bigint_str!("838083498911032969414721426845751663479194726707495046")
                ),
                (
                    "y",
                    bigint_str!("4310143708685312414132851373791311001152018708061750480")
                ),
                (
                    "value",
                    bigint_str!(
            "59479631769792988345961122678598249997181612138456851058217178025444564264149"
        )
                ),
                (
                    "new_x",
                    bigint_str!(
            "59479631769792988345961122678598249997181612138456851058217178025444564264149"
        )
                )
            ]
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_ec_double_assign_new_y_ok() {
        let hint_code = "value = new_y = (slope * (x - new_x) - y) % SECP_P";
        let mut vm = vm_with_range_check!();
        let mut exec_scopes = scope![
            (
                "slope",
                bigint_str!(
                    "48526828616392201132917323266456307435009781900148206102108934970258721901549"
                )
            ),
            (
                "x",
                bigint_str!("838083498911032969414721426845751663479194726707495046")
            ),
            (
                "new_x",
                bigint_str!(
                    "59479631769792988345961122678598249997181612138456851058217178025444564264149"
                )
            ),
            (
                "y",
                bigint_str!("4310143708685312414132851373791311001152018708061750480")
            )
        ];
        //Execute the hint
        assert_matches!(
            run_hint!(
                vm,
                HashMap::new(),
                hint_code,
                &mut exec_scopes,
                &[(
                    SECP_REM,
                    Felt252::one().shl(32_u32)
                        + Felt252::one().shl(9_u32)
                        + Felt252::one().shl(8_u32)
                        + Felt252::one().shl(7_u32)
                        + Felt252::one().shl(6_u32)
                        + Felt252::one().shl(4_u32)
                        + Felt252::one()
                )]
                .into_iter()
                .map(|(k, v)| (k.to_string(), v))
                .collect()
            ),
            Ok(())
        );

        check_scope!(
            &exec_scopes,
            [
                (
                    "value",
                    bigint_str!(
            "7948634220683381957329555864604318996476649323793038777651086572350147290350"
        )
                ),
                (
                    "new_y",
                    bigint_str!(
            "7948634220683381957329555864604318996476649323793038777651086572350147290350"
        )
                )
            ]
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_fast_ec_add_assign_new_x_ok() {
        let hint_code = "from starkware.cairo.common.cairo_secp.secp_utils import SECP_P, pack\n\nslope = pack(ids.slope, PRIME)\nx0 = pack(ids.point0.x, PRIME)\nx1 = pack(ids.point1.x, PRIME)\ny0 = pack(ids.point0.y, PRIME)\n\nvalue = new_x = (pow(slope, 2, SECP_P) - x0 - x1) % SECP_P";
        let mut vm = vm_with_range_check!();

        //Insert ids.point0, ids.point1.x and ids.slope into memory
        vm.segments = segments![
            //ids.point0
            ((1, 0), 89712),
            ((1, 1), 56),
            ((1, 2), 1233409),
            ((1, 3), 980126),
            ((1, 4), 10),
            ((1, 5), 8793),
            //ids.point0.x
            ((1, 6), 1235216451),
            ((1, 7), 5967),
            ((1, 8), 2171381),
            //ids.slope
            ((1, 9), 67470097831679799377177424_i128),
            ((1, 10), 43370026683122492246392730_i128),
            ((1, 11), 16032182557092050689870202_i128)
        ];

        //Initialize run_context
        run_context!(vm, 0, 20, 15);

        let ids_data = HashMap::from([
            ("point0".to_string(), HintReference::new_simple(-15)),
            ("point1".to_string(), HintReference::new_simple(-9)),
            ("slope".to_string(), HintReference::new_simple(-6)),
        ]);
        let mut exec_scopes = ExecutionScopes::new();

        //Execute the hint
        assert_matches!(
            run_hint!(
                vm,
                ids_data,
                hint_code,
                &mut exec_scopes,
                &[(
                    SECP_REM,
                    Felt252::one().shl(32_u32)
                        + Felt252::one().shl(9_u32)
                        + Felt252::one().shl(8_u32)
                        + Felt252::one().shl(7_u32)
                        + Felt252::one().shl(6_u32)
                        + Felt252::one().shl(4_u32)
                        + Felt252::one()
                )]
                .into_iter()
                .map(|(k, v)| (k.to_string(), v))
                .collect()
            ),
            Ok(())
        );

        check_scope!(
            &exec_scopes,
            [
                (
                    "value",
                    bigint_str!(
            "8891838197222656627233627110766426698842623939023296165598688719819499152657"
        )
                ),
                (
                    "new_x",
                    bigint_str!(
            "8891838197222656627233627110766426698842623939023296165598688719819499152657"
        )
                )
            ]
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_fast_ec_add_assign_new_y_ok() {
        let hint_code = "value = new_y = (slope * (x0 - new_x) - y0) % SECP_P";
        let mut vm = vm_with_range_check!();

        let mut exec_scopes = scope![
            (
                "slope",
                bigint_str!(
                    "48526828616392201132917323266456307435009781900148206102108934970258721901549"
                )
            ),
            (
                "x0",
                bigint_str!("838083498911032969414721426845751663479194726707495046")
            ),
            (
                "new_x",
                bigint_str!(
                    "59479631769792988345961122678598249997181612138456851058217178025444564264149"
                )
            ),
            (
                "y0",
                bigint_str!("4310143708685312414132851373791311001152018708061750480")
            )
        ];

        //Execute the hint
        assert_matches!(
            run_hint!(
                vm,
                HashMap::new(),
                hint_code,
                &mut exec_scopes,
                &[(
                    SECP_REM,
                    Felt252::one().shl(32_u32)
                        + Felt252::one().shl(9_u32)
                        + Felt252::one().shl(8_u32)
                        + Felt252::one().shl(7_u32)
                        + Felt252::one().shl(6_u32)
                        + Felt252::one().shl(4_u32)
                        + Felt252::one()
                )]
                .into_iter()
                .map(|(k, v)| (k.to_string(), v))
                .collect()
            ),
            Ok(())
        );

        check_scope!(
            &exec_scopes,
            [
                (
                    "value",
                    bigint_str!(
            "7948634220683381957329555864604318996476649323793038777651086572350147290350"
        )
                ),
                (
                    "new_y",
                    bigint_str!(
            "7948634220683381957329555864604318996476649323793038777651086572350147290350"
        )
                )
            ]
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_ec_mul_inner_ok() {
        let hint_code = "memory[ap] = (ids.scalar % PRIME) % 2";
        let mut vm = vm_with_range_check!();

        let scalar = 89712_i32;
        //Insert ids.scalar into memory
        vm.segments = segments![((1, 0), scalar)];

        //Initialize RunContext
        run_context!(vm, 0, 2, 1);

        let ids_data = ids_data!["scalar"];

        //Execute the hint
        assert_matches!(run_hint!(vm, ids_data, hint_code), Ok(()));

        //Check hint memory inserts
        check_memory![vm.segments.memory, ((1, 2), 0)];
    }

    #[test]
    fn get_ec_point_from_var_name_ok() {
        /*EcPoint {
            x: (1,2,3)
            y: (4,5,6)
        }*/
        let mut vm = vm!();
        vm.set_fp(1);
        vm.segments = segments![
            ((1, 0), 1),
            ((1, 1), 2),
            ((1, 2), 3),
            ((1, 3), 4),
            ((1, 4), 5),
            ((1, 5), 6)
        ];
        let ids_data = ids_data!["e"];
        let ap_tracking = ApTracking::default();
        let e = EcPoint::from_var_name("e", &vm, &ids_data, &ap_tracking).unwrap();
        assert_eq!(e.x.d0.as_ref(), &Felt252::one());
        assert_eq!(e.x.d1.as_ref(), &Felt252::from(2));
        assert_eq!(e.x.d2.as_ref(), &Felt252::from(3));
        assert_eq!(e.y.d0.as_ref(), &Felt252::from(4));
        assert_eq!(e.y.d1.as_ref(), &Felt252::from(5));
        assert_eq!(e.y.d2.as_ref(), &Felt252::from(6));
    }

    #[test]
    fn get_ec_point_from_var_name_missing_member() {
        /*EcPoint {
            x: (1,2,3)
            y: (4,_,_)
        }*/
        let mut vm = vm!();
        vm.set_fp(1);
        vm.segments = segments![((1, 0), 1), ((1, 1), 2), ((1, 2), 3), ((1, 3), 4)];
        let ids_data = ids_data!["e"];
        let ap_tracking = ApTracking::default();
        let r = EcPoint::from_var_name("e", &vm, &ids_data, &ap_tracking);
        assert_matches!(r, Err(HintError::IdentifierHasNoMember(x, y)) if x == "e.y" && y == "d1")
    }

    #[test]
    fn get_ec_point_from_var_name_invalid_reference() {
        let mut vm = vm!();
        vm.segments = segments![((1, 0), 1), ((1, 1), 2)];
        let ids_data = ids_data!["e"];
        let ap_tracking = ApTracking::default();
        let r = EcPoint::from_var_name("e", &vm, &ids_data, &ap_tracking);
        assert_matches!(r, Err(HintError::UnknownIdentifier(x)) if x == "e")
    }
}
