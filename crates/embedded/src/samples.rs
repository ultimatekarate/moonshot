// Stub: synthetic IMU trace for the embedded loop.
// Real implementation generates a realistic 100-sample trajectory at compile time.
pub const SAMPLES: [(f32, f32); 4] = [
    (0.0, 0.0),
    (0.1, 0.05),
    (0.2, 0.10),
    (0.3, 0.15),
];
