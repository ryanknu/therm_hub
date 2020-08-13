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
        let insert = diesel::insert_into(thermostats::table)
            .values(&new_thermostat);
        
        if cfg!(feature="queries") {
            println!("{}", diesel::debug_query::<diesel::pg::Pg, _>(&insert).to_string());
        }
        
        insert.get_result(connection).expect("Whoopsie-doodles")
    }

    pub fn query_dates(connection: &PgConnection, start_date: &DateTime<Utc>, end_date: &DateTime<Utc>) -> Result<Vec<Thermostat>, diesel::result::Error> {
        use thermostats::dsl;
        let start_date = start_date.naive_utc();
        let end_date = end_date.naive_utc();
        let query = dsl::thermostats
            .select((dsl::id, dsl::name, dsl::time, dsl::is_hygrostat, dsl::temperature, dsl::relative_humidity))
            .filter(dsl::time.ge(start_date))
            .filter(dsl::time.le(end_date));

        if cfg!(feature="queries") {
            println!("{}", diesel::debug_query::<diesel::pg::Pg, _>(&query).to_string());
        }
        
        query.load::<Thermostat>(connection)
    }
}