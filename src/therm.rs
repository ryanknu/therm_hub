#[derive(Debug, Serialize, Clone)]
pub struct Therm {
    name: String,
    time: String,
    temp: i32,
}

impl Therm {
    pub fn new(name: String, time: String, temp: i32) -> Therm {
        Therm {
            name,
            time,
            temp,
        }
    }
}