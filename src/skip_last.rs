// Copyright 2025 Redglyph
//

#[derive(Clone)]
pub struct SkipLast<I: Iterator<Item=T>, T: Sized> {
    iter: I,
    first: bool,
    cache: Option<T>
}

impl<I: Iterator<Item=T>, T: Sized> Iterator for SkipLast<I, T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.first {
            self.cache = self.iter.next();
            self.first = false;
        }
        let mut item = self.cache.take();
        if item.is_some() {
            self.cache = self.iter.next();
            if self.cache.is_none() {
                item = None;
            }
        }
        item
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let (lower, upper) = self.iter.size_hint();
        let new_lower = lower.saturating_sub(1);
        let new_upper = match upper {
            Some(n) => Some(n.saturating_sub(1)),
            None => None,
        };
        (new_lower, new_upper)
    }

    fn count(self) -> usize {
        if self.first {
            self.iter.count().saturating_sub(1)
        } else {
            self.iter.count()
        }
    }
}

pub trait SkipLastIterator<T>: Iterator<Item=T> {
    /// Creates an iterator that skips the last element.
    fn skip_last(self) -> SkipLast<Self, T> where Self: Sized {
        SkipLast { iter: self, first: true, cache: None }
    }
}

impl<I: Iterator<Item=T>, T: Sized> SkipLastIterator<T> for I {}

// ---------------------------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super:: *;

    #[test]
    fn basic() {
        let v1 = vec![10, 20, 30, 40];
        let result1 = v1.iter().enumerate().skip_last().map(|(i, x)| format!("{i}:{x}")).collect::<Vec<_>>().join(",");
        assert_eq!(result1, "0:10,1:20,2:30");
        let result2 = v1.into_iter().enumerate().skip_last().map(|(i, x)| format!("{i}:{x}")).collect::<Vec<_>>().join(",");
        assert_eq!(result2, "0:10,1:20,2:30");

        let v2 = vec![10];
        let mut iter2 = v2.into_iter().skip_last();
        assert_eq!(iter2.next(), None);
        assert_eq!(iter2.next(), None);

        let v3: Vec<i32> = vec![];
        let mut iter3 = v3.iter().skip_last();
        assert_eq!(iter3.next(), None);
        assert_eq!(iter3.next(), None);

        let mut v4 = vec![10, 20, 30, 400];
        for x in v4.iter_mut().skip_last() {
            *x *= 10;
        }
        assert_eq!(v4, vec![100, 200, 300, 400]);
    }

    #[test]
    fn size_hint() {
        let iter = (0..usize::MAX).skip_last();
        let hint = iter.size_hint();
        assert_eq!(hint, (usize::MAX - 1, Some(usize::MAX - 1)));
    }

    #[test]
    fn count() {
        let iter1 = (0..usize::MAX).skip_last();
        assert_eq!(iter1.count(), usize::MAX - 1);

        let mut iter2 = (0..usize::MAX).skip_last();
        iter2.next();
        assert_eq!(iter2.count(), usize::MAX - 2);

        let iter3 = (0..0).skip_last();
        assert_eq!(iter3.count(), 0);

        let mut iter4 = (0..0).skip_last();
        iter4.next();
        assert_eq!(iter4.count(), 0);
    }
}