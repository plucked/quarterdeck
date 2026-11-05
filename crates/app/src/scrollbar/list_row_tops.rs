use std::rc::Rc;

use gpui::{ListOffset, Pixels, px};

/// Where each row of a virtualized list starts, estimated by the list's
/// owner, and where the last one ends. A list that measures only the rows it
/// shows does not know its own height; these estimates size and place the
/// scrollbar's thumb instead. Cheap to clone.
#[derive(Debug, Clone)]
pub struct ListRowTops(Rc<[f32]>);

impl ListRowTops {
    /// From each row's estimated height, in order.
    pub fn from_heights(heights: impl IntoIterator<Item = f32>) -> Self {
        // The running sum: the top of every row, then the end of the last.
        let mut tops = vec![0.];
        let mut top = 0.;
        for height in heights {
            top += height;
            tops.push(top);
        }
        Self(tops.into())
    }

    /// The estimated height of every row together.
    pub fn total(&self) -> Pixels {
        px(self.end())
    }

    /// How far down the list `offset` is.
    pub fn y_of(&self, offset: ListOffset) -> Pixels {
        let top = self.0.get(offset.item_ix).copied().unwrap_or(self.end());
        px(top) + offset.offset_in_item
    }

    /// The row at `y` and how far into it `y` is; the last row past the end.
    pub fn offset_at(&self, y: Pixels) -> ListOffset {
        let row_count = self.0.len() - 1;
        if row_count == 0 {
            return ListOffset::default();
        }
        let y = f32::from(y).max(0.);
        let row = self
            .0
            .partition_point(|top| *top <= y)
            .saturating_sub(1)
            .min(row_count - 1);
        ListOffset {
            item_ix: row,
            offset_in_item: px(y - self.0[row]),
        }
    }

    fn end(&self) -> f32 {
        self.0.last().copied().unwrap_or(0.)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A row and an offset into it map to a height and back.
    #[test]
    fn maps_offsets_and_heights_both_ways() {
        let tops = ListRowTops::from_heights([10., 20., 30.]);
        assert_eq!(tops.total(), px(60.));
        let offset = ListOffset {
            item_ix: 1,
            offset_in_item: px(5.),
        };
        assert_eq!(tops.y_of(offset), px(15.));
        let back = tops.offset_at(px(15.));
        assert_eq!((back.item_ix, back.offset_in_item), (1, px(5.)));
    }

    /// Heights past the end land in the last row; an empty list stays at the top.
    #[test]
    fn clamps_to_the_rows() {
        let tops = ListRowTops::from_heights([10., 20.]);
        assert_eq!(tops.offset_at(px(500.)).item_ix, 1);
        assert_eq!(tops.offset_at(px(-5.)).item_ix, 0);
        assert_eq!(ListRowTops::from_heights([]).offset_at(px(5.)).item_ix, 0);
    }
}
