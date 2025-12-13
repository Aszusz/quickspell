use std::sync::Mutex;

pub struct Counter {
    count: Mutex<i32>,
}

impl Default for Counter {
    fn default() -> Self {
        Self {
            count: Mutex::new(0),
        }
    }
}

impl Counter {
    pub fn get(&self) -> i32 {
        *self.count.lock().expect("counter mutex poisoned")
    }

    pub fn increment(&self) -> i32 {
        let mut count = self.count.lock().expect("counter mutex poisoned");
        *count += 1;
        *count
    }

    pub fn decrement(&self) -> i32 {
        let mut count = self.count.lock().expect("counter mutex poisoned");
        *count -= 1;
        *count
    }

    pub fn reset(&self) -> i32 {
        let mut count = self.count.lock().expect("counter mutex poisoned");
        *count = 0;
        *count
    }
}
