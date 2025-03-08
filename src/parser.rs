use crate::utils::*;

#[derive(Debug, Clone, PartialEq)]
enum FormulaType {
    SleepConst,
    SleepRef,
    Constant,
    Reference,
    ConstantConstant,
    ConstantReference,
    ReferenceConstant,
    ReferenceReference,
    RangeFunction,
    InvalidFormula,
}