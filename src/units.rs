pub fn gain_from_db(decibels: f32) -> f32 {
    (10.0f32).powf(decibels * 0.05)
}

pub fn db_from_gain(gain: f32) -> f32 {
    gain.max(0.0).log(10.0) * 20.0
}

pub fn to_range(bottom: f32, top: f32, x: f32) -> f32 {
    x * (top - bottom) + bottom
}

pub fn from_range(bottom: f32, top: f32, x: f32) -> f32 {
    (x - bottom) / (top - bottom)
}

pub fn mix(x: f32, y: f32, a: f32) -> f32 {
    x * (1.0 - a) + y * a
}

pub fn clamp(x: f32, min_v: f32, max_v: f32) -> f32 {
    (x).min(max_v).max(min_v)
}

pub fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    // Scale, bias and saturate x to 0..1 range
    let x = clamp((x - edge0) / (edge1 - edge0), 0.0, 1.0);
    // Evaluate polynomial
    x * x * (3.0 - 2.0 * x)
}
