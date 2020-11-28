
#[derive(Copy, Clone)]
pub struct Stats<T> {
    pub total: T,
    pub min: T,
    pub max: T,
    pub count: T,
    pub timeout: bool,
}

impl<T: num::Integer + num::Bounded + Copy> Default for Stats<T> {
    fn default() -> Self {
        let zero = T::zero();
        let max = T::max_value();
        let result = Stats::<T> {
            total: zero,
            min: max,
            max: zero,
            count: zero,
            timeout: false,
        };
        result
    }
}

impl<T> Stats<T>
where
    T: num::Integer + num::Bounded + std::ops::AddAssign + Copy, // + std::ops::Add<Output = T>
{
    pub fn average(&self) -> Option<T> {
        if self.count == T::zero() { return None; } else {return Some(self.total / self.count)}
    }

    pub fn update(&mut self, value: Option<T>) {
        if let Some(value) = value {
            if value < self.min {
                self.min = value;
            }
            if value > self.max {
                self.max = value
            }
            self.count = self.count + T::one();
            self.total += value;
        } else {
            self.timeout = true;
        }
    }

    pub fn _clear(&mut self) {
        let zero = T::zero();
        self.total = zero;
        self.min = T::max_value();
        self.max = zero;
        self.count = zero;
        self.timeout = false;
    }
}
