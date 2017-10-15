extern crate serde;
#[macro_use(Serialize, Deserialize)]
extern crate serde_derive;
extern crate serde_yaml;

mod _cargo {
  struct IndexEntry {
    pub name: String,
    pub vers: String,
    pub deps: Vec<DependencyEntry>,
    pub cksum: String,
    pub features: FeatureEntry
    pub yanked: bool
  }

  struct DependencyEntry {
    pub name: String,
    pub req: String,
    pub features: Vec<String>,
    pub optional: bool,
    pub default_features: bool,
    pub target: Optional<String>,
    pub kind: String,
  }

  struct FeatureEntry {
    pub features_per_
}

#[cfg(test)]
mod tests {
  #[test]
  fn it_works() {
  }
}
