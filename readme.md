# ThermHub API

## Running the Server
1. Install Postgres through your package manager.
2. Copy `.env.example` to `.env`, or optionally set environment variables.
3. Set offline to yes if you want to test with stubbed data.
4. Run `cargo run`

## How to use the EcoBee API
1. Put your client ID in `ECOBEE_CLIENT_ID` environment variable.
2. Call `/install/1`.
3. Put the 4-digit `ecobee_pin` into the ecobee.com portal.
4. Call `/install/2?code={code}` with the `code` you received in step 1.