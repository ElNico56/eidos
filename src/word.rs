use derive_more::{Display, From};
use enum_iterator::Sequence;
use serde::Deserialize;

use crate::{field::*, function::*};

#[derive(
    Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Hash, From, Sequence, Deserialize,
)]
pub enum Word {
    // Numbers
    Ti,
    Tu,
    Ta,
    Te,
    // Scalars
    Seva,
    Sevi,
    Me,
    // Vectors
    Kova,
    Kovi,
    // Inputs
    Le,
    Po,
    Lusa,
    Mesi,
    // Outputs
    Ke,
    // Operators
    Ma,
    Sa,
    Na,
    Reso,
    Solo,
    // Controls
    Sila,
    Vila,
    Pa,
    Pi,
    // Combinators
    No,
    Mo,
    Re,
    Rovo,
}

impl Word {
    pub fn function(&self) -> Function {
        use Word::*;
        match self {
            Ti => Nullary::One.into(),
            Tu => Nullary::Two.into(),
            Ta => Nullary::Five.into(),
            Te => Nullary::Ten.into(),
            Kova => Nullary::OneX.into(),
            Kovi => Nullary::OneY.into(),
            Seva => Nullary::X.into(),
            Sevi => Nullary::Y.into(),
            Me => Nullary::Filter.into(),
            Le => ScalarInputFieldKind::Elevation.into(),
            Po => ScalarInputFieldKind::Density.into(),
            Lusa => ScalarInputFieldKind::Light.into(),
            Mesi => ScalarInputFieldKind::Magic.into(),
            Ke => VectorOutputFieldKind::Gravity.into(),
            Ma => HomoBinOp::Add.into(),
            Sa => HeteroBinOp::Mul.into(),
            Na => MathUnOp::Neg.into(),
            Solo => ToScalarOp::Magnitude.into(),
            Reso => ScalarUnOp::Reciprocal.into(),
            No => Combinator1::Drop.into(),
            Mo => Combinator1::Duplicate.into(),
            Re => Combinator2::Swap.into(),
            Rovo => Combinator2::Over.into(),
            Sila => ControlKind::XSlider.into(),
            Vila => ControlKind::YSlider.into(),
            Pa => Nullary::TargetX.into(),
            Pi => Nullary::TargetY.into(),
        }
    }
    pub fn cost(&self) -> f32 {
        use Word::*;
        match self {
            Ti => 1.0,
            Tu => 2.0,
            Ta => 5.0,
            Te => 10.0,
            Sila => 2.0,
            Vila => 2.0,
            Pa => 3.0,
            Pi => 3.0,
            _ => 1.0,
        }
    }
}
