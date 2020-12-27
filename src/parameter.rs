use vst::util::AtomicFloat;

fn to_range(x: f32, bottom: f32, top: f32) -> f32 {
    x * (top - bottom) + bottom
}

fn from_range(x: f32, bottom: f32, top: f32) -> f32 {
    (x - bottom) / (top - bottom)
}

pub struct Parameter {
    name: String,
    normalized_value: AtomicFloat,
    value: AtomicFloat,
    pub default: f32,
    pub min: f32,
    pub max: f32,
    display_func: fn(f32) -> String,
}

impl Parameter {
    pub fn new(
        name: &str,
        default: f32,
        min: f32,
        max: f32,
        display_func: fn(f32) -> String,
    ) -> Parameter {
        Parameter {
            name: String::from(name),
            normalized_value: AtomicFloat::new(from_range(default, min, max)),
            value: AtomicFloat::new(default),
            default,
            min,
            max,
            display_func,
        }
    }

    pub fn get_normalized(&self) -> f32 {
        self.normalized_value.get()
    }

    pub fn set_normalized(&self, x: f32) {
        self.normalized_value.set(x);
        self.value.set(to_range(x, self.min, self.max));
    }

    pub fn get(&self) -> f32 {
        self.value.get()
    }

    pub fn set(&self, x: f32) {
        self.value.set(x);
        self.normalized_value.set(from_range(x, self.min, self.max));
    }

    pub fn get_display(&self) -> String {
        (self.display_func)(self.value.get())
    }

    pub fn get_name(&self) -> String {
        self.name.clone()
    }
}
