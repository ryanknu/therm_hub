CREATE TABLE ecobee_token (
  id SERIAL PRIMARY KEY,
  access_token VARCHAR NOT NULL,
  refresh_token VARCHAR NOT NULL,
  expires TIMESTAMP NOT NULL
)