//! Pure value-level operations shared by the reference interpreter and bytecode VM.
use super::lexer::K;
use super::value::V;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum RuntimeOpError {
    Type {
        message: String,
        position: (usize, usize),
    },
    Runtime {
        message: String,
        position: (usize, usize),
    },
}

pub(super) fn finite(number: f64, position: (usize, usize)) -> Result<V, RuntimeOpError> {
    if number.is_finite() {
        Ok(V::Num(number))
    } else {
        Err(RuntimeOpError::Runtime {
            message: "Numeric operation produced a non-finite result.".to_owned(),
            position,
        })
    }
}

pub(super) fn unary(
    value: V,
    operator: K,
    operand_position: (usize, usize),
    operator_position: (usize, usize),
) -> Result<V, RuntimeOpError> {
    match operator {
        K::Minus => match value {
            V::Num(number) => finite(-number, operator_position),
            _ => Err(RuntimeOpError::Type {
                message: "Unary '-' expects Number.".to_owned(),
                position: operand_position,
            }),
        },
        K::Bang => match value {
            V::Bool(boolean) => Ok(V::Bool(!boolean)),
            _ => Err(RuntimeOpError::Type {
                message: "'!' expects Boolean.".to_owned(),
                position: operand_position,
            }),
        },
        _ => unreachable!("parser only constructs supported unary operators"),
    }
}

pub(super) fn binary(
    left: V,
    operator: K,
    right: V,
    positions: ((usize, usize), (usize, usize), (usize, usize)),
) -> Result<V, RuntimeOpError> {
    let (operator_position, left_position, right_position) = positions;
    match operator {
        K::Plus => match (left, right) {
            (V::Num(left), V::Num(right)) => finite(left + right, operator_position),
            (V::Str(left), V::Str(right)) => Ok(V::Str(left + &right)),
            (left, right) => Err(RuntimeOpError::Runtime {
                message: format!(
                    "'+' expects two Numbers or two Strings, got {} and {}.",
                    left.ty(),
                    right.ty()
                ),
                position: operator_position,
            }),
        },
        K::Minus | K::Star | K::Slash | K::Percent => {
            let (left, right) = match (left, right) {
                (V::Num(left), V::Num(right)) => (left, right),
                (left, _) if !matches!(&left, V::Num(_)) => {
                    return Err(RuntimeOpError::Type {
                        message: "Arithmetic operators require Number operands.".to_owned(),
                        position: left_position,
                    });
                }
                (_, _) => {
                    return Err(RuntimeOpError::Type {
                        message: "Arithmetic operators require Number operands.".to_owned(),
                        position: right_position,
                    });
                }
            };
            if (operator == K::Slash || operator == K::Percent) && right == 0.0 {
                return Err(RuntimeOpError::Runtime {
                    message: "Division or modulo by zero.".to_owned(),
                    position: operator_position,
                });
            }
            finite(
                match operator {
                    K::Minus => left - right,
                    K::Star => left * right,
                    K::Slash => left / right,
                    K::Percent => left % right,
                    _ => unreachable!(),
                },
                operator_position,
            )
        }
        K::Gt | K::GtEq | K::Lt | K::LtEq => {
            let (left, right) = match (left, right) {
                (V::Num(left), V::Num(right)) => (left, right),
                (left, _) if !matches!(&left, V::Num(_)) => {
                    return Err(RuntimeOpError::Type {
                        message: "Ordering comparisons require Number operands.".to_owned(),
                        position: left_position,
                    });
                }
                (_, _) => {
                    return Err(RuntimeOpError::Type {
                        message: "Ordering comparisons require Number operands.".to_owned(),
                        position: right_position,
                    });
                }
            };
            Ok(V::Bool(match operator {
                K::Gt => left > right,
                K::GtEq => left >= right,
                K::Lt => left < right,
                K::LtEq => left <= right,
                _ => unreachable!(),
            }))
        }
        K::EqEq => Ok(V::Bool(left.eq(&right))),
        K::BangEq => Ok(V::Bool(!left.eq(&right))),
        _ => unreachable!("parser only constructs supported binary operators"),
    }
}
