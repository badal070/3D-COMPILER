use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MathValue {
    Real(f64),
    Complex(ComplexValue),
    Integer(i64),
    Rational(i64, i64),
    Vector(MathVector),
    Matrix(MathMatrix),
    Boolean(bool),
    Undefined,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MathType {
    Real,
    Complex,
    Integer,
    Rational,
    Vector(usize),
    Matrix(usize, usize),
    Boolean,
    Function(Vec<MathType>, Box<MathType>),
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ComplexValue {
    pub real: f64,
    pub imag: f64,
}

impl ComplexValue {
    pub fn new(real: f64, imag: f64) -> Self {
        Self { real, imag }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MathVector {
    pub elements: Vec<f64>,
}

impl MathVector {
    pub fn new(elements: Vec<f64>) -> Self {
        Self { elements }
    }

    pub fn dimension(&self) -> usize {
        self.elements.len()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MathMatrix {
    pub rows: usize,
    pub cols: usize,
    pub elements: Vec<Vec<f64>>,
}

impl MathMatrix {
    pub fn new(elements: Vec<Vec<f64>>) -> Self {
        let rows = elements.len();
        let cols = elements.first().map_or(0, |row| row.len());
        Self {
            rows,
            cols,
            elements,
        }
    }

    pub fn is_rectangular(&self) -> bool {
        self.elements.iter().all(|row| row.len() == self.cols)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BoundType {
    Inclusive,
    Exclusive,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Interval {
    pub start: f64,
    pub end: f64,
    pub start_bound: BoundType,
    pub end_bound: BoundType,
}

impl Interval {
    pub fn closed(start: f64, end: f64) -> Self {
        Self {
            start,
            end,
            start_bound: BoundType::Inclusive,
            end_bound: BoundType::Inclusive,
        }
    }

    pub fn open(start: f64, end: f64) -> Self {
        Self {
            start,
            end,
            start_bound: BoundType::Exclusive,
            end_bound: BoundType::Exclusive,
        }
    }

    pub fn contains(&self, value: f64) -> bool {
        let left = match self.start_bound {
            BoundType::Inclusive => value >= self.start,
            BoundType::Exclusive => value > self.start,
        };
        let right = match self.end_bound {
            BoundType::Inclusive => value <= self.end,
            BoundType::Exclusive => value < self.end,
        };
        left && right
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MathSpace {
    Real,
    RealN(usize),
    Complex,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariableInterval {
    pub variable: String,
    pub interval: Interval,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Domain {
    pub space: MathSpace,
    pub variable_intervals: Vec<VariableInterval>,
    pub constraints: Vec<String>,
}

impl Domain {
    pub fn new(space: MathSpace) -> Self {
        Self {
            space,
            variable_intervals: Vec::new(),
            constraints: Vec::new(),
        }
    }

    pub fn with_interval(mut self, variable: impl Into<String>, interval: Interval) -> Self {
        self.variable_intervals.push(VariableInterval {
            variable: variable.into(),
            interval,
        });
        self
    }

    pub fn with_constraint(mut self, constraint: impl Into<String>) -> Self {
        self.constraints.push(constraint.into());
        self
    }
}
