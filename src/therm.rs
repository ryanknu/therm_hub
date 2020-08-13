use chrono::{DateTime, NaiveDateTime, Utc};
use diesel::prelude::*;
use diesel::PgConnection;
use super::schema::thermostats;

#[derive(Debug, Serialize, Clone, Queryable)]
pub struct Thermostat {
    pub id: i32,
    pub name: String,
    time: NaiveDateTime,
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
    pub fn time(&self) -> DateTime<Utc> {
        DateTime::<Utc>::from_utc(self.time, Utc)
    }

    pub fn new(name: String, time: DateTime<Utc>, temp: i32) -> Thermostat {
        Thermostat {
            id: 0,
            name,
            time: time.naive_utc(),
            temperature: temp,
            is_hygrostat: false,
            relative_humidity: 0,
        }
    }

    pub fn new2(name: String, time: DateTime<Utc>, is_hygrostat: bool, temperature: i32, relative_humidity: i32) -> Thermostat {
        Thermostat { id: 0, name, time: time.naive_utc(), is_hygrostat, temperature, relative_humidity }
    }

    pub fn insert(&self, connection: &PgConnection) -> Thermostat {
        let new_thermostat = NewThermostat {
            name: self.name.clone(),
            time: self.time,
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