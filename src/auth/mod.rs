//! Authentication sub-modules.
//!
//! The auth flow works as follows:
//!   1. `login` opens the user's browser to the backend OAuth endpoint.
//!   2. `server` starts a temporary local HTTP server on port 8080 to receive
//!      the redirect callback containing the JWT and username.
//!   3. `token` reads the saved JWT from disk for use in subsequent requests.
//!   4. `account` fetches and displays user info from the backend.
//!   5. `logout` deletes the stored credentials file.

pub mod account;
pub mod login;
pub mod logout;
pub mod server;
pub mod token;
