use std::ops::{Add, Sub};
use num_traits::*;

use crate::error::*;

/// Stable alternative to `std::iter::Step`
pub trait StableStep: Clone + PartialOrd<Self> + PrimInt + FromPrimitive {
    #[inline]
    fn forward(&self, count: usize) -> Option<Self> {
        self.checked_add(&Self::from_usize(count)?)
    }
    #[inline]
    fn backward(&self, count: usize) -> Option<Self> {
        self.checked_sub(&Self::from_usize(count)?)
    }
}
impl<T: PrimInt + FromPrimitive> StableStep for T {}

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
      let number: usize = split[0].parse().map_err(ErrorKind::BadRangeNumber)?;
      Ok(BidirectionalRange::new(number, number + 1))
    } else if split.len() == 2 {
      let start: usize = split[0].parse().map_err(ErrorKind::BadRangeNumber)?;
      let end: usize = split[1].parse().map_err(ErrorKind::BadRangeNumber)?;
      if start < end {
        Ok(BidirectionalRange::new(start, end + 1))
      } else {
        Ok(BidirectionalRange::new(start, end - 1))
      }
    } else {
      Err(ErrorKind::BadRange.into())
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

impl<A: StableStep + Copy> Iterator for BidirectionalRange<A>
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
        if (self.start < self.end && c.forward(1)? == self.end) || c.backward(1)? == self.end {
          return None;
        }
        c
      },
      None => {
        self.current = Some(self.start);
        return self.current;
      }
    };
    if self.start < self.end {
      self.current = Some(current.forward(1).unwrap());
    } else {
      self.current = Some(current.backward(1).unwrap());
    }
    self.current
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
