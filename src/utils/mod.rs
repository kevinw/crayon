//! Commonly used utilities like pools, variants and so on.

#[macro_use]
pub mod handle;
pub mod handle_pool;
pub mod object_pool;
pub mod hash;
pub mod variant;

mod finally;

pub use self::handle::{Handle, HandleIndex};
pub use self::handle_pool::{HandlePool, HandleIter};
pub use self::object_pool::ObjectPool;
pub use self::finally::{finally, finally_with};
pub use self::hash::{hash, HashValue};
pub use self::variant::{VariantChar, VariantStr};