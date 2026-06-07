#[derive(Default)]
pub struct Register<T> {
    value: T,
    to_latch_value: T,
    is_write_enabled: bool,
}

impl<T> Register<T>
where
    T: Copy + Default,
{
    pub fn new() -> Self {
        Self {
            value: T::default(),
            to_latch_value: T::default(),
            is_write_enabled: false,
        }
    }

    pub fn get(&self) -> T {
        self.value
    }

    pub fn set(&mut self, to_latch_value: T) {
        self.to_latch_value = to_latch_value;
    }

    pub fn set_write(&mut self, is_enabled: bool) {
        self.is_write_enabled = is_enabled;
    }

    pub fn tick(&mut self) {
        if self.is_write_enabled {
            self.value = self.to_latch_value;
        } else {
            self.to_latch_value = self.value;
        }

        self.is_write_enabled = false;
    }
}
