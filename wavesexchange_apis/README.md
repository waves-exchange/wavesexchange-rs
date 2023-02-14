# WX APIs

API clients to the Waves Exchange's REST APIs.

## Testing

### Integration tests

This crate contains long-running integrations tests that access external resources such as external REST APIs.
These tests are ignored by default, so that a regular `cargo test` invocation runs fast:
```shell
cargo test
```
```text
. . . . .
test api_clients_integration::state::defo_assets_list ... ignored, because variable INTEGRATION not found
test api_clients_integration::state::single_asset_price_request ... ignored, because variable INTEGRATION not found
test api_clients_integration::state::test_get_state ... ignored, because variable INTEGRATION not found

test result: ok. 0 passed; 0 failed; 10 ignored; 0 measured; 0 filtered out; finished in 0.00s
. . . . .
```

To enable these tests, set the `INTEGRATION` env variable to something non-empty:
```shell
INTEGRATION=1 cargo test
```
```text
. . . . .
test api_clients_integration::state::defo_assets_list ... ok
test api_clients_integration::state::test_get_state ... ok
test api_clients_integration::assets::test_assets_get ... ok

test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 22.07s
. . . . .
```
Note much longer execution time here.

**Important notice.** Currently, the `INTEGRATION` env variable must be set at compile time, not at runtime.
That means, once build, the test executable will work the same way regardless of the mentioned environment variable.
A simple consequence of that, if one tries to run `cargo test` (which builds tests with ITs ignored)
and then `INTEGRATION=1 cargo test` right after it (which will not rebuild because no source files were modified)
the result of the second invocation will be exactly the same (no ITs have run).
This is the limitation of the `test_with` crate and the Rust's test harness.
To work around this, just `touch` any source file before running tests again.

To run *only* the integration tests, and nothing else, use the following command:
```shell
INTEGRATION=1 cargo test --tests
```

### Doc tests

This crate also contains doctests which are being run by default, and also can be run separately with the following command:
```shell
cargo test --doc
```
```text
running 2 tests
test src/clients/http.rs - clients::http::HttpClient (line 13) - compile ... ok
test src/clients/http.rs - clients::http::WXRequestHandler (line 170) - compile ... ok
. . . . .
```

These tests also take up a considerable time because they are run sequentially one by one.
To run just unit tests, without doc tests (and without integration tests), use command:
```shell
cargo test --lib
```
Especially this command is useful if run in the workspace directory for all crates in this repository.
