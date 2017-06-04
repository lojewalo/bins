use std::ops::{Add, Sub};
use std::iter::Step;

use error::*;

#[derive(Debug, Clone)]
pub struct BidirectionalRange<Idx> {
  start: Idx,
  end: Idx,
  current: Option<Idx>
}

impl<Idx> BidirectionalRange<Idx> {
  pub fn new(start: Idx, end: Idx) -> Self {
    BidirectionalRange {
      start: start,
      end: end,
      current: None
    }
  }

  pub fn parse_usize(string: &str) -> Result<BidirectionalRange<usize>> {
    let split: Vec<&str> = string.split('-').collect();
    if split.len() != 2 {
      let number: usize = split[0].parse().map_err(Some).map_err(BinsError::InvalidRange)?;
      Ok(BidirectionalRange::new(number, number + 1))
    } else if split.len() == 2 {
      let start: usize = split[0].parse().map_err(Some).map_err(BinsError::InvalidRange)?;
      let end: usize = split[1].parse().map_err(Some).map_err(BinsError::InvalidRange)?;
      if start < end {
        Ok(BidirectionalRange::new(start, end + 1))
      } else {
        Ok(BidirectionalRange::new(start, end - 1))
      }
    } else {
      Err(BinsError::InvalidRange(None))
    }
  }
}

impl<Idx> BidirectionalRange<Idx>
  where Idx: PartialOrd<Idx>
{
  pub fn contains(&self, item: Idx) -> bool {
    if self.start < self.end {
      item >= self.start && item < self.end
    } else {
      item <= self.start && item > self.end
    }
  }
}

impl<A: Step + Clone> Iterator for BidirectionalRange<A>
  where for<'a> &'a A: Add<&'a A, Output=A>,
        for<'a> &'a A: Sub<&'a A, Output=A>
{
  type Item = A;

  fn next(&mut self) -> Option<Self::Item> {
    if self.start == self.end {
      return None;
    }
    let current = match self.current.take() {
      Some(c) => {
        if (self.start < self.end && c.add_one() == self.end) || c.sub_one() == self.end {
          return None;
        }
        c
      },
      None => {
        self.current = Some(self.start.clone());
        return self.current.clone();
      }
    };
    if self.start < self.end {
      self.current = Some(current.add_one());
    } else {
      self.current = Some(current.sub_one());
    }
    self.current.clone()
  }
}

pub trait AnyContains<Idx> {
  fn any_contains(&self, i: Idx) -> bool;
}

impl<Idx> AnyContains<Idx> for Vec<BidirectionalRange<Idx>>
  where Idx: PartialOrd + Copy
{
  fn any_contains(&self, i: Idx) -> bool {
    self.iter().any(|r| r.contains(i))
  }
}
