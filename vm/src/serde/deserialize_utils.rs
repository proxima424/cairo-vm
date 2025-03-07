use crate::stdlib::{fmt, num::ParseIntError, prelude::*, str::FromStr};

use crate::{
    serde::deserialize_program::{OffsetValue, ValueAddress},
    types::instruction::Register,
};
use felt::{Felt252, ParseFeltError};
use nom::{
    branch::alt,
    bytes::{
        complete::{take_till, take_until},
        streaming::tag,
    },
    character::complete::digit1,
    combinator::{map_res, opt, value},
    error::{Error, ErrorKind, ParseError},
    sequence::{delimited, tuple},
    Err, IResult,
};
use num_integer::Integer;

#[derive(Debug, PartialEq, Eq)]
pub enum ReferenceParseError {
    IntError(ParseIntError),
    Felt252Error(ParseFeltError),
    InvalidStringError(String),
}

impl fmt::Display for ReferenceParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ReferenceParseError::IntError(error) => {
                write!(f, "Int parsing error: ")?;
                error.fmt(f)
            }
            ReferenceParseError::Felt252Error(error) => {
                write!(f, "Felt252 parsing error: ")?;
                error.fmt(f)
            }
            ReferenceParseError::InvalidStringError(error) => {
                write!(f, "Invalid reference string error: ")?;
                error.fmt(f)
            }
        }
    }
}

// Checks if the hex string has an odd length.
// If that is the case, prepends '0' to it.
pub fn maybe_add_padding(mut hex: String) -> String {
    if hex.len().is_odd() {
        hex.insert(0, '0');
        return hex;
    }
    hex
}

/*
NOM PARSERS
See the docs for context and grammar explanation:
cleopatra_cairo/docs/references_parsing/README.md
*/

// Checks if the input has outer brackets. This is used to set
// the `dereference` field of ValueAddress.
fn outer_brackets(input: &str) -> IResult<&str, bool> {
    opt(delimited(
        tag("["),
        take_until_unbalanced('[', ']'),
        tag("]"),
    ))(input)
    .map(|(rem_input, res_opt)| {
        if let Some(res) = res_opt {
            (res, true)
        } else {
            (rem_input, false)
        }
    })
}

// Removes the cast string and parenthesis from the value.
fn take_cast(input: &str) -> IResult<&str, &str> {
    let (rem_input, _) = tag("cast")(input)?;
    delimited(tag("("), take_until_unbalanced('(', ')'), tag(")"))(rem_input)
        .map(|(rem_input, res)| (res, rem_input))
}

// Returns the first argument of the cast function from the value.
fn take_cast_first_arg(input: &str) -> IResult<&str, &str> {
    let (rem_input, _) = take_cast(input)?;

    take_until(",")(rem_input).map(|(rem_input, res)| (res, rem_input))
}

fn register(input: &str) -> IResult<&str, Option<Register>> {
    opt(alt((
        value(Register::AP, tag("ap")),
        value(Register::FP, tag("fp")),
    )))(input)
}

fn offset(input: &str) -> IResult<&str, i32> {
    if input.eq("") {
        return Ok(("", 0));
    }

    let (rem_input, sign) = opt(alt((tag(" + "), tag(" - "))))(input)?;
    let (rem_input, num_opt) = opt(delimited(tag("("), take_until(")"), tag(")")))(rem_input)?;

    let sign = if let Some(" - ") = sign {
        -1_i32
    } else {
        1_i32
    };

    if let Some(num) = num_opt {
        let parsed_num: i32 = num
            .parse()
            .map_err(|_| Err::Error(ParseError::from_error_kind(num, ErrorKind::MapRes)))?;

        Ok((rem_input, sign * parsed_num))
    } else {
        let (rem_input, parsed_num) = map_res(digit1, i32::from_str)(rem_input)?;

        Ok((rem_input, sign * parsed_num))
    }
}

fn register_and_offset(input: &str) -> IResult<&str, (Option<Register>, i32)> {
    let (rem_input, reg) = register(input)?;
    let (rem_input, offset) = offset(rem_input)?;

    Ok((rem_input, (reg, offset)))
}

fn inner_dereference(input: &str) -> IResult<&str, OffsetValue> {
    if input.is_empty() {
        return Ok(("", OffsetValue::Value(0)));
    }
    let (input, _sign) = opt(alt((tag(" + "), tag(" - "))))(input)?;

    map_res(
        delimited(tag("["), take_until("]"), tag("]")),
        register_and_offset,
    )(input)
    .map(|(rem_input, res)| {
        let (_, (register, offset)) = res;
        let offset_value = match register {
            None => OffsetValue::Value(offset),
            Some(reg) => OffsetValue::Reference(reg, offset, true),
        };
        (rem_input, offset_value)
    })
}

fn no_inner_dereference(input: &str) -> IResult<&str, OffsetValue> {
    let (rem_input, (register, offset)) = register_and_offset(input)?;
    let offset_value = match register {
        None => OffsetValue::Value(offset),
        Some(reg) => OffsetValue::Reference(reg, offset, false),
    };
    Ok((rem_input, offset_value))
}

pub fn parse_value(input: &str) -> IResult<&str, ValueAddress> {
    let (rem_input, (dereference, second_arg, fst_offset, snd_offset)) = tuple((
        outer_brackets,
        take_cast_first_arg,
        opt(alt((inner_dereference, no_inner_dereference))),
        opt(alt((inner_dereference, no_inner_dereference))),
    ))(input)?;

    let (indirection_level, (_, struct_)) =
        tuple((tag(", "), take_till(|c: char| c == '*')))(second_arg)?;

    let type_: String = if let Some(indirections) = indirection_level.get(1..) {
        struct_.to_string() + indirections
    } else {
        struct_.to_string()
    };

    let fst_offset = fst_offset.unwrap_or(OffsetValue::Value(0));
    let snd_offset = snd_offset.unwrap_or(OffsetValue::Value(0));

    // cast to big int if necessary
    let (offset1, offset2) = if struct_ == "felt" && indirection_level.is_empty() {
        let offset1 = match fst_offset {
            OffsetValue::Immediate(imm) => OffsetValue::Immediate(imm),
            OffsetValue::Value(val) => OffsetValue::Immediate(Felt252::new(val)),
            OffsetValue::Reference(reg, val, refe) => OffsetValue::Reference(reg, val, refe),
        };

        let offset2 = match snd_offset {
            OffsetValue::Immediate(imm) => OffsetValue::Immediate(imm),
            OffsetValue::Value(val) => OffsetValue::Immediate(Felt252::new(val)),
            OffsetValue::Reference(reg, val, refe) => OffsetValue::Reference(reg, val, refe),
        };

        (offset1, offset2)
    } else {
        (fst_offset, snd_offset)
    };

    let value_address = ValueAddress {
        offset1,
        offset2,
        dereference,
        value_type: type_,
    };

    Ok((rem_input, value_address))
}

/// A parser similar to `nom::bytes::complete::take_until()`, except that this
/// one does not stop at balanced opening and closing tags. It is designed to
/// work inside the `nom::sequence::delimited()` parser.
///
/// # Basic usage
/// ```
/// use nom::bytes::complete::tag;
/// use nom::sequence::delimited;
/// use cairo_take_until_unbalanced::take_until_unbalanced;
///
/// let mut parser = delimited(tag("<"), take_until_unbalanced('<', '>'), tag(">"));
/// assert_eq!(parser("<<inside>inside>abc"), Ok(("abc", "<inside>inside")));
/// ```
/// It skips nested brackets until it finds an extra unbalanced closing bracket. Escaped brackets
/// like `\<` and `\>` are not considered as brackets and are not counted. This function is
/// very similar to `nom::bytes::complete::take_until(">")`, except it also takes nested brackets.
/// NOTE: trimmed down from https://docs.rs/parse-hyperlinks to fix bugs. The project itself seems
/// abandonned.
pub fn take_until_unbalanced(
    opening_bracket: char,
    closing_bracket: char,
) -> impl Fn(&str) -> IResult<&str, &str> {
    move |i: &str| {
        let mut index = 0;
        let mut bracket_counter = 0;
        while let Some(n) = &i
            .get(index..)
            .ok_or_else(|| Err::Error(Error::from_error_kind(i, ErrorKind::TakeUntil)))?
            .find(&[opening_bracket, closing_bracket, '\\'][..])
        {
            index += n;
            let mut it = i
                .get(index..)
                .ok_or_else(|| Err::Error(Error::from_error_kind(i, ErrorKind::TakeUntil)))?
                .chars();
            match it.next().unwrap_or_default() {
                c if c == '\\' => {
                    // Skip the escape char `\`.
                    index += '\\'.len_utf8();
                    // Skip also the following char.
                    let c = it.next().unwrap_or_default();
                    index += c.len_utf8();
                }
                c if c == opening_bracket => {
                    bracket_counter += 1;
                    index += opening_bracket.len_utf8();
                }
                c if c == closing_bracket => {
                    // Closing bracket.
                    bracket_counter -= 1;
                    index += closing_bracket.len_utf8();
                }
                // Can not happen.
                _ => unreachable!(),
            };
            // We found the unmatched closing bracket.
            if bracket_counter == -1 {
                // We do not consume it.
                index -= closing_bracket.len_utf8();
                let remaining = i
                    .get(index..)
                    .ok_or_else(|| Err::Error(Error::from_error_kind(i, ErrorKind::TakeUntil)))?;
                let matching = i
                    .get(0..index)
                    .ok_or_else(|| Err::Error(Error::from_error_kind(i, ErrorKind::TakeUntil)))?;
                return Ok((remaining, matching));
            };
        }

        if bracket_counter == 0 {
            Ok(("", i))
        } else {
            Err(Err::Error(Error::from_error_kind(i, ErrorKind::TakeUntil)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stdlib::string::ToString;
    use num_traits::{One, Zero};

    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::*;

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn outer_brackets_test() {
        let deref_value = "[cast([fp])]";
        let parsed_deref = outer_brackets(deref_value);
        assert_eq!(parsed_deref, Ok(("cast([fp])", true)));

        let ref_value = "cast([fp])";
        let parsed_ref = outer_brackets(ref_value);
        assert_eq!(parsed_ref, Ok(("cast([fp])", false)));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn take_cast_test() {
        let value = "cast([fp + (-1)], felt*)";
        let parsed = take_cast(value);
        assert_eq!(parsed, Ok(("[fp + (-1)], felt*", "")));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn take_cast_first_arg_test() {
        let value = "cast([fp + (-1)] + (-1), felt*)";
        let parsed = take_cast_first_arg(value);

        assert_eq!(parsed, Ok(("[fp + (-1)] + (-1)", ", felt*")));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn parse_register_test() {
        let value = "fp + (-1)";
        let parsed = register(value);

        assert_eq!(parsed, Ok((" + (-1)", Some(Register::FP))));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn parse_offset_test() {
        let value_1 = " + (-1)";
        let parsed_1 = offset(value_1);
        assert_eq!(parsed_1, Ok(("", -1_i32)));

        let value_2 = " + 1";
        let parsed_2 = offset(value_2);
        assert_eq!(parsed_2, Ok(("", 1_i32)));

        let value_3 = " - 1";
        let parsed_3 = offset(value_3);
        assert_eq!(parsed_3, Ok(("", -1_i32)));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn parse_register_and_offset_test() {
        let value_1 = "fp + 1";
        let parsed_1 = register_and_offset(value_1);

        assert_eq!(parsed_1, Ok(("", (Some(Register::FP), 1_i32))));

        let value_2 = "ap + (-1)";
        let parsed_2 = register_and_offset(value_2);

        assert_eq!(parsed_2, Ok(("", (Some(Register::AP), -1_i32))));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn parse_inner_dereference_test() {
        let value = "[fp + (-1)] + 2";
        let parsed = inner_dereference(value);

        assert_eq!(
            parsed,
            Ok((" + 2", OffsetValue::Reference(Register::FP, -1_i32, true)))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn parse_no_inner_dereference_test() {
        let value = "ap + 3";
        let parsed = no_inner_dereference(value);

        assert_eq!(
            parsed,
            Ok(("", OffsetValue::Reference(Register::AP, 3_i32, false)))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn parse_value_with_inner_dereference_test() {
        let value = "[cast([fp + (-1)] + 2, felt*)]";
        let parsed = parse_value(value);

        assert_eq!(
            parsed,
            Ok((
                "",
                ValueAddress {
                    offset2: OffsetValue::Value(2),
                    offset1: OffsetValue::Reference(Register::FP, -1_i32, true),
                    dereference: true,
                    value_type: "felt".to_string(),
                }
            ))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn parse_value_with_no_inner_dereference_test() {
        let value = "cast(ap + 2, felt*)";
        let parsed = parse_value(value);

        assert_eq!(
            parsed,
            Ok((
                "",
                ValueAddress {
                    offset1: OffsetValue::Reference(Register::AP, 2_i32, false),
                    offset2: OffsetValue::Value(0),
                    dereference: false,
                    value_type: "felt".to_string(),
                }
            ))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn parse_value_with_no_register_test() {
        let value = "cast(825323, felt*)";
        let parsed = parse_value(value);
        assert_eq!(
            parsed,
            Ok((
                "",
                ValueAddress {
                    offset1: OffsetValue::Value(825323),
                    offset2: OffsetValue::Value(0),
                    dereference: false,
                    value_type: "felt".to_string(),
                }
            ))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn parse_value_with_no_inner_deref_and_two_offsets() {
        let value = "[cast(ap - 0 + (-1), felt*)]";
        let parsed = parse_value(value);

        assert_eq!(
            parsed,
            Ok((
                "",
                ValueAddress {
                    offset1: OffsetValue::Reference(Register::AP, 0_i32, false),
                    offset2: OffsetValue::Value(-1),
                    dereference: true,
                    value_type: "felt".to_string(),
                }
            ))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn parse_value_with_inner_deref_and_offset2() {
        let value = "[cast([ap] + 1, __main__.felt*)]";
        let parsed = parse_value(value);

        assert_eq!(
            parsed,
            Ok((
                "",
                ValueAddress {
                    offset1: OffsetValue::Reference(Register::AP, 0_i32, true),
                    offset2: OffsetValue::Value(1),
                    dereference: true,
                    value_type: "__main__.felt".to_string(),
                }
            ))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn parse_value_with_inner_deref_and_immediate() {
        let value = "[cast([ap] + 1, felt)]";
        let parsed = parse_value(value);

        assert_eq!(
            parsed,
            Ok((
                "",
                ValueAddress {
                    offset1: OffsetValue::Reference(Register::AP, 0_i32, true),
                    offset2: OffsetValue::Immediate(Felt252::one()),
                    dereference: true,
                    value_type: "felt".to_string(),
                }
            ))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn parse_value_with_inner_deref_to_pointer() {
        let value = "[cast([ap + 1] + 1, felt*)]";
        let parsed = parse_value(value);

        assert_eq!(
            parsed,
            Ok((
                "",
                ValueAddress {
                    offset1: OffsetValue::Reference(Register::AP, 1_i32, true),
                    offset2: OffsetValue::Value(1),
                    dereference: true,
                    value_type: "felt".to_string(),
                }
            ))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn parse_value_with_2_inner_deref() {
        let value = "[cast([ap] + [fp + 1], __main__.felt*)]";
        let parsed = parse_value(value);

        assert_eq!(
            parsed,
            Ok((
                "",
                ValueAddress {
                    offset1: OffsetValue::Reference(Register::AP, 0_i32, true),
                    offset2: OffsetValue::Reference(Register::FP, 1_i32, true),
                    dereference: true,
                    value_type: "__main__.felt".to_string(),
                }
            ))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn parse_value_with_2_inner_dereferences() {
        let value = "[cast([ap + 1] + [fp + 1], __main__.felt*)]";
        let parsed = parse_value(value);

        assert_eq!(
            parsed,
            Ok((
                "",
                ValueAddress {
                    offset1: OffsetValue::Reference(Register::AP, 1_i32, true),
                    offset2: OffsetValue::Reference(Register::FP, 1_i32, true),
                    dereference: true,
                    value_type: "__main__.felt".to_string(),
                }
            ))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn parse_value_with_no_reference() {
        let value = "cast(825323, felt)";
        let parsed = parse_value(value);

        assert_eq!(
            parsed,
            Ok((
                "",
                ValueAddress {
                    offset1: OffsetValue::Immediate(Felt252::new(825323_i32)),
                    offset2: OffsetValue::Immediate(Felt252::zero()),
                    dereference: false,
                    value_type: "felt".to_string(),
                }
            ))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn parse_value_with_one_reference() {
        let value = "[cast([ap] + 1, starkware.cairo.common.cairo_secp.ec.EcPoint*)]";
        let parsed = parse_value(value);

        assert_eq!(
            parsed,
            Ok((
                "",
                ValueAddress {
                    offset1: OffsetValue::Reference(Register::AP, 0_i32, true),
                    offset2: OffsetValue::Value(1),
                    dereference: true,
                    value_type: "starkware.cairo.common.cairo_secp.ec.EcPoint".to_string(),
                }
            ))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn parse_value_with_doble_reference() {
        let value = "[cast([ap] + 1, starkware.cairo.common.cairo_secp.ec.EcPoint**)]";
        let parsed = parse_value(value);

        assert_eq!(
            parsed,
            Ok((
                "",
                ValueAddress {
                    offset1: OffsetValue::Reference(Register::AP, 0_i32, true),
                    offset2: OffsetValue::Value(1),
                    dereference: true,
                    value_type: "starkware.cairo.common.cairo_secp.ec.EcPoint*".to_string(),
                }
            ))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn parse_value_to_felt_with_doble_reference() {
        let value = "[cast([ap] + [ap], felt)]";
        let parsed = parse_value(value);

        assert_eq!(
            parsed,
            Ok((
                "",
                ValueAddress {
                    offset1: OffsetValue::Reference(Register::AP, 0_i32, true),
                    offset2: OffsetValue::Reference(Register::AP, 0_i32, true),
                    dereference: true,
                    value_type: "felt".to_string(),
                }
            ))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn parse_value_to_felt_with_doble_reference_and_offsets() {
        let value = "[cast([ap + 1] + [ap + 2], felt)]";
        let parsed = parse_value(value);

        assert_eq!(
            parsed,
            Ok((
                "",
                ValueAddress {
                    offset1: OffsetValue::Reference(Register::AP, 1_i32, true),
                    offset2: OffsetValue::Reference(Register::AP, 2_i32, true),
                    dereference: true,
                    value_type: "felt".to_string(),
                }
            ))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn test_take_until_unmatched() {
        assert_eq!(take_until_unbalanced('(', ')')("abc"), Ok(("", "abc")));
        assert_eq!(
            take_until_unbalanced('(', ')')("url)abc"),
            Ok((")abc", "url"))
        );
        assert_eq!(
            take_until_unbalanced('(', ')')("u()rl)abc"),
            Ok((")abc", "u()rl"))
        );
        assert_eq!(
            take_until_unbalanced('(', ')')("u(())rl)abc"),
            Ok((")abc", "u(())rl"))
        );
        assert_eq!(
            take_until_unbalanced('(', ')')("u(())r()l)abc"),
            Ok((")abc", "u(())r()l"))
        );
        assert_eq!(
            take_until_unbalanced('(', ')')("u(())r()labc"),
            Ok(("", "u(())r()labc"))
        );
        assert_eq!(
            take_until_unbalanced('(', ')')(r#"u\((\))r()labc"#),
            Ok(("", r#"u\((\))r()labc"#))
        );
        assert_eq!(
            take_until_unbalanced('(', ')')("u(())r(labc"),
            Err(nom::Err::Error(nom::error::Error::new(
                "u(())r(labc",
                ErrorKind::TakeUntil
            )))
        );
        assert_eq!(
            take_until_unbalanced('€', 'ü')("€uü€€üürlüabc"),
            Ok(("üabc", "€uü€€üürl"))
        );
        assert_eq!(
            take_until_unbalanced('(', ')')("u(())r()labc\\"),
            Err(nom::Err::Error(nom::error::Error::new(
                "u(())r()labc\\",
                ErrorKind::TakeUntil
            )))
        );
        assert_eq!(
            take_until_unbalanced('(', ')')("u\\rl)abc"),
            Ok((")abc", "u\\rl"))
        );
        assert_eq!(
            take_until_unbalanced('(', ')')("u\\\\rl)abc"),
            Ok((")abc", "u\\\\rl"))
        );
        // 'µ' used to check for escaped multi-byte character
        assert_eq!(
            take_until_unbalanced('(', ')')("u\\µrl)"),
            Ok((")", "u\\µrl"))
        );
        assert_eq!(
            take_until_unbalanced('(', ')')("u\\µ)rl"),
            Ok((")rl", "u\\µ"))
        );
        assert_eq!(
            take_until_unbalanced('(', ')')("urlabc\\"),
            Err(nom::Err::Error(nom::error::Error::new(
                "urlabc\\",
                ErrorKind::TakeUntil
            )))
        );
        assert_eq!(take_until_unbalanced('(', ')')("abc"), Ok(("", "abc")));
        assert_eq!(
            take_until_unbalanced('(', ')')("(abc"),
            Err(nom::Err::Error(nom::error::Error::new(
                "(abc",
                ErrorKind::TakeUntil
            )))
        );
    }
}
