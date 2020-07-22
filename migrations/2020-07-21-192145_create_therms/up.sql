CREATE TABLE thermostats (
  id SERIAL PRIMARY KEY,
  name VARCHAR NOT NULL,
  time TIMESTAMP NOT NULL,
  is_hygrostat BOOLEAN NOT NULL,
  temperature INT NOT NULL,
  relative_humidity INT NOT NULL
)