use chrono::NaiveDateTime;
use diesel::prelude::*;
use diesel::PgConnection;
use super::schema::thermostats;

#[derive(Debug, Serialize, Clone, Queryable)]
pub struct Thermostat {
    pub id: i32,
    pub name: String,
    pub time: NaiveDateTime,
    pub is_hygrostat: bool,
    pub temperature: i32,
    pub relative_humidity: i32,
}

#[derive(Insertable)]
#[table_name="thermostats"]
struct NewThermostat {
    pub name: String,
    pub time: NaiveDateTime,
    pub is_hygrostat: bool,
    pub temperature: i32,
    pub relative_humidity: i32,
}

impl Thermostat {
    pub fn new(name: String, time: String, temp: i32) -> Thermostat {
        Thermostat {
            id: 0,
            name,
            time: NaiveDateTime::parse_from_str(&time, "%Y-%m-%dT%H:%M:%S%z").unwrap(),
            temperature: temp,
            is_hygrostat: false,
            relative_humidity: 0,
        }
    }

    pub fn insert(&self, connection: &PgConnection) -> Thermostat {
        let new_thermostat = NewThermostat {
            name: self.name.clone(),
            time: self.time.clone(),
            is_hygrostat: self.is_hygrostat,
            temperature: self.temperature,
            relative_humidity: self.relative_humidity,
        };
        diesel::insert_into(thermostats::table)
            .values(&new_thermostat)
            .get_result(connection)
            .expect("Whoopsie-doodles")
    }
}