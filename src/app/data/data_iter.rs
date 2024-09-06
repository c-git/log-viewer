use super::{Data, LogRow};

#[derive(Debug)]
pub struct DataIter<'a> {
    pos: usize,
    data: &'a Data,
}

impl<'a> DataIter<'a> {
    pub fn new(data: &'a Data) -> Self {
        Self { pos: 0, data }
    }
}

impl<'a> Iterator for DataIter<'a> {
    type Item = &'a LogRow;

    fn next(&mut self) -> Option<Self::Item> {
        let result = match self.data.filtered_rows.as_ref() {
            Some(filtered) => {
                if self.pos >= filtered.len() {
                    return None;
                }
                Some(&self.data.rows[filtered[self.pos]])
            }
            None => {
                if self.pos >= self.data.rows.len() {
                    return None;
                }
                Some(&self.data.rows[self.pos])
            }
        };
        self.pos += 1;
        result
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        if n < self.pos {
            return None;
        }
        self.pos = n;
        self.next()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(clippy::iter_nth_zero)]
    fn nth_no_reverse() {
        let row0 = super::super::tests::create_log_row_no_extra();
        let row1 = super::super::tests::create_log_row_with_extra();
        let data = Data {
            rows: vec![row0.clone(), row1.clone()],
            ..Default::default()
        };
        let mut iter = data.rows_iter();
        assert_eq!(iter.nth(0), Some(&row0));
        assert_eq!(iter.nth(0), None);
        assert_eq!(iter.nth(1), Some(&row1));
        assert_eq!(iter.nth(1), None);
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn next_works() {
        let row0 = super::super::tests::create_log_row_no_extra();
        let row1 = super::super::tests::create_log_row_with_extra();
        let data = Data {
            rows: vec![row0.clone(), row1.clone()],
            ..Default::default()
        };
        let mut iter = data.rows_iter();
        assert_eq!(iter.next(), Some(&row0));
        assert_eq!(iter.next(), Some(&row1));
        assert_eq!(iter.next(), None);
    }
}