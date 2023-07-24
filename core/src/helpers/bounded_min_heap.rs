use std::{cmp::Reverse, collections::BinaryHeap};

pub trait HeapItem: Ord + Clone {
  fn get_heap_key(&self) -> f64;
}

pub struct BoundedMinHeap<T: HeapItem> {
  heap: BinaryHeap<Reverse<T>>,
  capacity: usize,
}

impl<T: HeapItem> BoundedMinHeap<T> {
  pub fn new(capacity: usize) -> Self {
    Self {
      heap: BinaryHeap::new(),
      capacity,
    }
  }

  pub fn push(&mut self, item: T) {
    if self.heap.len() < self.capacity {
      self.heap.push(Reverse(item));
    } else {
      let mut min = self.heap.peek_mut().unwrap();
      if item.get_heap_key() > (min.0).get_heap_key() {
        *min = Reverse(item);
      }
    }
  }

  pub fn pop(&mut self) -> Option<T> {
    self.heap.pop().map(|x| x.0)
  }

  pub fn peek(&self) -> Option<&T> {
    self.heap.peek().map(|x| &x.0)
  }
}
