## 2020-08-22
- Refactored the worker thread.
- Initial data pulls are executed and complete before background thread/http server are spawned.
- Retries added to weather.gov and database connection routines.
- Release notes API should work in production now!
- Added a method you can poke from the API to start a background photos refresh.
- Removed version API. You can now get the version from the /release-notes endpoint, in a header.
- Background photos will no longer be queried in offline mode.
## 2020-08-18
- Background photos
- Shared secrets
- Optimizations like you wouldn't believe, some of which made the server slower!
## 2020-08-13
- `/past` now works. Have fun searching past data.
- Time zones? What are those? Server now converts everything in and out to UTC.
- Offline env variable has been reworked as `#cfg`.
- Service file added for server deploys
## 2020-07-31
- License added.
- /time endpoint added, to get the current time on devices without an RTC.
- /install/1 and /install/2 endpoints added, to get started with the EcoBee API.
- Started refactoring HTTP functions into reusable functions.
## 2020-07-22
- Fix opening lots of connections to the DB.
## 2020-07-21
- Removed extra dotenv syncs
- Records are now written to the DB.
- EcoBee API started, mock results added.
- Added release notes command to return this file!
## 2020-07-20
- First release