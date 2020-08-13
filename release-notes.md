## 2020-08-12
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