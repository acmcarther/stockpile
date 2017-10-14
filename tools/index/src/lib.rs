extern crate serde;
#[macro_use(Serialize, Deserialize)]
extern crate serde_derive;
extern crate serde_yaml;

mod _cargo {
  struct IndexEntry {
    pub name: String,

  }
}

#[cfg(test)]
mod tests {
  #[test]
  fn it_works() {
  }
}
