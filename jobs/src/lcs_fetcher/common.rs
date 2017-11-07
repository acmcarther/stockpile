#[derive(Debug, Clone, Eq, PartialEq, Hash)]
/** A unique key for a downloable crate artifact */
pub struct CrateKey {
  pub name: String,
  pub version: String,
}

