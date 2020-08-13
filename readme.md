# ThermHub API

## Running the Server
1. Install Postgres through your package manager.
2. Copy `.env.example` to `.env`, or optionally set environment variables.
4. Run `cargo run` to live API's. Use `cargo run --features offline` to run with stubbed data.

## How to use the EcoBee API
1. Put your client ID in `ECOBEE_CLIENT_ID` environment variable.
2. Call `/install/1`.
3. Put the 4-digit `ecobee_pin` into the ecobee.com portal.
4. Call `/install/2?code={code}` with the `code` you received in step 1.

## Build for Linux on MacOS
```
mkdir target
mkdir target/release
docker build -t th .
docker create --name extract th
docker cp extract:/usr/src/myapp/target/release/therm_hub target/release/therm_hub
docker rm extract
```

## Running on Linux
1. Build for linux and SCP to server
2. Install and harden NGINX, proxy_pass incoming requests to port 3000
3. Symlink `/etc/systemd/system/hub.service` to `hub.service` (or copy it)
4. Reload system control daemon `systemctl reload-daemon`
5. Enable the hub service `systemctl enable hub`
6. Start the hub service `systemctl start hub`