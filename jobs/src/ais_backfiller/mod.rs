use super::Job;
use git2;
use std::io;

#[derive(Builder)]
pub struct AisBackfillerJob {
}

impl Default for AisBackfillerJob {
  fn default() -> AisBackfillerJob {
    AisBackfillerJob {
    }
  }
}

#[derive(Debug)]
pub enum AisBackfillErr {
  GitErr(git2::Error),
  IoErr(io::Error),
}
define_from_error_boilerplate!(io::Error, AisBackfillErr, AisBackfillErr::IoErr);
define_from_error_boilerplate!(git2::Error, AisBackfillErr, AisBackfillErr::GitErr);

impl AisBackfillerJob {
  fn run_now(&mut self) -> Result<(), AisBackfillErr> {
    Ok(())
  }
}

impl Job for AisBackfillerJob {
  fn run(&mut self) {
    self.run_now().unwrap()
  }
}


pub mod testing {
}


#[cfg(test)]
mod tests {
}
