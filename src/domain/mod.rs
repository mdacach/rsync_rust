pub use delta::*;
pub use patch::*;
pub use signature::*;

pub mod delta;
// Delta is the representation of a difference from `basis_file` and  `updated_file``
pub mod patch;
// Patch is the process of applying a Delta to `basis_file` and constructing `recreated_file`
pub mod signature; // Signature is the representation of `basis_file`
