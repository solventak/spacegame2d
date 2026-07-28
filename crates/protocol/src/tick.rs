#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Tick(pub u64);

impl Tick {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub fn increment(self, amount: Tick) -> Self {
        Self(self.0.saturating_add(amount.0))
    }
}

impl From<u64> for Tick {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

impl From<Tick> for u64 {
    fn from(value: Tick) -> Self {
        value.0
    }
}

impl std::ops::Add for Tick {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        self.increment(rhs)
    }
}

impl std::ops::Sub for Tick {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0.saturating_sub(rhs.0))
    }
}
