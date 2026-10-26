pub mod factory;
pub mod local;
pub mod traits;
#[cfg(feature = "remote")]
pub mod remote;

pub use factory::create;
pub use traits::{BackendError, CompilerBackend};
#[cfg(feature = "remote")]
pub use remote::{GodboltBackend, GodboltClient};
