extern crate log;
extern crate serde;
#[macro_use(Serialize, Deserialize)]
extern crate serde_derive;
extern crate serde_yaml;
#[macro_use]
extern crate zcfg_flag_parser;
extern crate chrono;
extern crate fern;

use chrono::DateTime;
use chrono::Utc;
use std::env;
use zcfg_flag_parser::FlagParser;

pub mod cargo {
  use super::*;
  use std::collections::HashMap;
  // Mostly a copy from github/rust-lang/crates.io/src/git.rs
  // WARNING: On sync from upstream crates.io-index, all modifications
  // besides the "extra" entry are lost. Add new metadata into "extra".
  #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
  pub struct IndexEntry {
    pub name: String,
    pub vers: String,
    pub deps: Vec<DependencyEntry>,
    pub cksum: String,
    pub features: HashMap<String, Vec<String>>,
    pub yanked: Option<bool>,
  }

  // Mostly a copy from github/rust-lang/crates.io/src/git.rs
  #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
  pub struct DependencyEntry {
    pub name: String,
    pub req: String,
    pub features: Vec<String>,
    pub optional: bool,
    pub default_features: bool,
    pub target: Option<String>,
    pub kind: Option<String>,
  }

  // Stockpile-specific data added to the index entry
  #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
  pub struct ExtraEntry {
    dev_dependencies: Option<Vec<DependencyEntry>>,
  }

  // Unique identifier for a Cargo crate
  #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
  pub struct CrateKey {
    pub name: String,
    pub version: String,
  }
}

pub mod configuration {
  #[derive(Debug, Clone, Serialize, Deserialize)]
  pub struct WorkspaceConfiguration {
    crate_sets: Vec<MaintainerConfiguration>,
    skip_dev_dependencies: Vec<String>,
  }

  #[derive(Debug, Clone, Serialize, Deserialize)]
  pub struct MaintainerConfiguration {
    maintainer: String,
    crates: Vec<String>,
  }
}

pub mod snapshot {
  #[derive(Debug, Clone, Serialize, Deserialize)]
  pub struct WorkspaceSnapshot {
    version: String,
    members: Vec<String>,
    details: Vec<CrateSnapshot>,
  }

  #[derive(Debug, Clone, Serialize, Deserialize)]
  pub struct CrateSnapshot {
    name: String,
    version: String,
    maintainer: String,
    dependencies: String,
    resolution_type: Option<ResolutionType>,
  }

  #[derive(Debug, Clone, Serialize, Deserialize)]
  pub struct ResolutionType {
    crates_io: Option<bool>,
    git: Option<GitResolution>,
  }

  #[derive(Debug, Clone, Serialize, Deserialize)]
  pub struct GitResolution {
    repository: String,
    revision: String,
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceMetadata {
  last_index_time: Option<DateTime<Utc>>,
  crates_io_index_revision: String,
}


pub mod iter_util {
  use std::fmt::Debug;

  pub fn aggregate_results<O, E: Debug>(left: Result<Vec<O>, E>, right: Result<Vec<O>, E>) -> Result<Vec<O>, E> {
    if left.is_err() {
      return left
    }
    if right.is_err() {
      return right
    }

    let mut l_inner = left.unwrap();
    let mut r_inner = right.unwrap();
    l_inner.append(&mut r_inner);
    Ok(l_inner)
  }
}

/** Initializes all of the things. */
pub fn init() {
  init_flags();
  init_logger();
}

/** Initializes zcfg flags for the binary. */
pub fn init_flags() {
  FlagParser::new().parse_from_args(env::args().skip(1)).unwrap();
}

/** Initializes the Fern logger for the binary. */
pub fn init_logger() {
  fern::Dispatch::new()
    .format(|out, message, record| {
      out.finish(format_args!("{}[{}][{}] {}",
          chrono::Local::now()
              .format("[%Y-%m-%d][%H:%M:%S]"),
          record.target(),
          record.level(),
          message))
    })
    .level(log::LogLevelFilter::Debug)
    .chain(std::io::stdout())
    .apply()
    .unwrap();
}

#[macro_export]
macro_rules! define_box_clone_boilerplate {
  ($original_ty:tt, $clone_ty:tt) => {
    define_box_clone_boilerplate_inner!($original_ty, $original_ty, $clone_ty, $clone_ty);
  }
}

#[macro_export]
macro_rules! define_box_clone_boilerplate_inner {
  ($original_ty:ty, $original_ident:ident, $clone_ty:ty, $clone_ident:ident) => {
    #[allow(non_camel_case_types)]
    pub trait $clone_ident {
      fn clone_box(&self) -> Box<$original_ty>;
    }

    impl<T> $clone_ty for T where T: 'static + $original_ident + Clone {
      fn clone_box(&self) -> Box<$original_ty> {
        Box::new(self.clone())
      }
    }

    impl Clone for Box<$original_ty> {
      fn clone(&self) -> Box<$original_ty> {
        self.clone_box()
      }
    }
  }
}

#[macro_export]
macro_rules! define_from_error_boilerplate {
  ($source_ty:ty, $dest_ty:ty, $dest_constructor:expr) => {
    impl From<$source_ty> for $dest_ty {
      fn from(error: $source_ty) -> $dest_ty {
        $dest_constructor(error)
      }
    }
  }
}

#[cfg(test)]
mod tests {
  #[test]
  fn it_works() {
  }
}
