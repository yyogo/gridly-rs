use core::fmt::Debug;
use core::iter::FusedIterator;
use core::marker::PhantomData;
use core::ops::Index;

use crate::grid::{BoundsError, GridBounds};
use crate::location::component::{
    ColumnRangeError, Range as IndexRange, RangeError, RowRangeError,
};
use crate::location::{Column, Component as LocComponent, Location, Range as LocationRange, Row};

pub trait BaseGrid: GridBounds {
    type Item;

    /// Get a reference to a cell, without doing bounds checking. Implementors
    /// of this method are allowed to assume that bounds checking has already
    /// been performed on the location.
    unsafe fn get_unchecked(&self, loc: &Location) -> &Self::Item;
}

impl<G: BaseGrid> BaseGrid for &G {
    type Item = G::Item;

    unsafe fn get_unchecked(&self, loc: &Location) -> &Self::Item {
        (**self).get_unchecked(loc)
    }
}

impl<G: BaseGrid> BaseGrid for &mut G {
    type Item = G::Item;

    unsafe fn get_unchecked(&self, loc: &Location) -> &Self::Item {
        (**self).get_unchecked(loc)
    }
}

/// View methods for a Grid, aimed at providing support for iterating over rows,
/// columns, and cells inside of those views.
pub trait Grid: BaseGrid {
    /// Get a reference to a cell in a grid. Returns an error if the location
    /// is out of bounds with the specific boundary violation.
    fn get(&self, location: impl Into<Location>) -> Result<&Self::Item, BoundsError> {
        self.check_location(location)
            .map(move |loc| unsafe { self.get_unchecked(&loc) })
    }

    // Get a view of a grid, over its rows or columns
    fn view<T: LocComponent>(&self) -> View<Self, T> {
        View::new(self)
    }

    /// Get a view of a grid's rows
    fn rows(&self) -> RowsView<Self> {
        self.view()
    }

    /// Get a view of a grid's columns
    fn columns(&self) -> ColumnsView<Self> {
        self.view()
    }

    /// Get a view of a single row or column in a grid, without bounds checking that
    /// row or column index.
    unsafe fn single_view_unchecked<T: LocComponent>(&self, index: T) -> SingleView<Self, T> {
        SingleView::new_unchecked(self, index)
    }

    /// Get a view of a single row in a grid, without bounds checking that row's index
    unsafe fn row_unchecked(&self, row: impl Into<Row>) -> RowView<Self> {
        self.single_view_unchecked(row.into())
    }

    /// Get a view of a single column in a grid, without bounds checking that column's index
    unsafe fn column_unchecked(&self, column: impl Into<Column>) -> ColumnView<Self> {
        self.single_view_unchecked(column.into())
    }

    /// Get a view of a single row or column in a grid. Returns an error if the index of the
    /// row or column is out of bounds for the grid.
    fn single_view<T: LocComponent>(&self, index: T) -> Result<SingleView<Self, T>, RangeError<T>> {
        SingleView::new(self, index)
    }

    /// Get a view of a single row in a grid. Returns an error if the index of the row is
    /// out of bounds for the grid.
    fn row(&self, row: impl Into<Row>) -> Result<RowView<Self>, RowRangeError> {
        self.single_view(row.into())
    }

    /// Get a view of a single column in a grid. Returns an error if the index of the column
    /// is out of bounds for the grid.
    fn column(&self, column: impl Into<Column>) -> Result<ColumnView<Self>, ColumnRangeError> {
        self.single_view(column.into())
    }
}

impl<G: BaseGrid> Grid for G {}

/// A view of the Rows or Columns in a grid.
///
/// This struct provides a row- or column-major view of a grid. For instance,
/// a `View<MyGrid, Row>` is a View of all of the rows in MyGrid.
///
///
pub struct View<'a, G: Grid + ?Sized, T: LocComponent> {
    grid: &'a G,
    index: PhantomData<T>,
}

impl<'a, G: Grid + ?Sized, T: LocComponent> View<'a, G, T> {
    fn new(grid: &'a G) -> Self {
        Self {
            grid,
            index: PhantomData,
        }
    }

    pub unsafe fn get_unchecked(&self, index: T) -> SingleView<G, T> {
        SingleView::new_unchecked(self.grid, index)
    }

    pub fn get(&self, index: impl Into<T>) -> Result<SingleView<G, T>, RangeError<T>> {
        SingleView::new(self.grid, index.into())
    }

    pub fn range(&self) -> IndexRange<T> {
        self.grid.range()
    }

    pub fn iter(
        &self,
    ) -> impl Iterator<Item = SingleView<'a, G, T>>
                 + DoubleEndedIterator
                 + FusedIterator
                 + ExactSizeIterator
                 + Debug
                 + Clone {
        let grid = self.grid;
        self.range()
            .map(move |index| unsafe { SingleView::new_unchecked(grid, index) })
    }
}

// TODO: impl Index for GridView. Requires Higher Kinded Lifetimes, because
// Index currently requires an &'a T, but we want to return a GridSingleView<'a, T>
// TODO: IntoIterator

pub type RowsView<'a, G> = View<'a, G, Row>;
pub type ColumnsView<'a, G> = View<'a, G, Column>;

// Implementor notes: a GridSingleView's index field is guaranteed to have been
// bounds checked against the grid. Therefore, we provide unsafe constructors, and
// then freely use unsafe accessors in the safe interface.
pub struct SingleView<'a, G: Grid + ?Sized, T: LocComponent> {
    grid: &'a G,
    index: T,
}

impl<'a, G: Grid + ?Sized, T: LocComponent> SingleView<'a, G, T> {
    unsafe fn new_unchecked(grid: &'a G, index: T) -> Self {
        Self { grid, index }
    }

    fn new(grid: &'a G, index: T) -> Result<Self, RangeError<T>> {
        grid.check_component(index)
            .map(move |index| unsafe { Self::new_unchecked(grid, index) })
    }

    pub fn index(&self) -> T {
        self.index
    }

    pub unsafe fn get_unchecked(&self, cross: T::Converse) -> &'a G::Item {
        self.grid.get_unchecked(&self.index.combine(cross))
    }

    pub fn get(
        &self,
        cross: impl Into<T::Converse>,
    ) -> Result<&'a G::Item, RangeError<T::Converse>> {
        self.grid
            .check_component(cross.into())
            .map(move |cross| unsafe { self.get_unchecked(cross) })
    }

    /// Get the locations associated with this view
    pub fn range(&self) -> LocationRange<T> {
        LocationRange::new(self.index, self.grid.range())
    }

    pub fn iter(
        &self,
    ) -> impl Iterator<Item = &'a G::Item>
                 + DoubleEndedIterator
                 + FusedIterator
                 + ExactSizeIterator
                 + Debug
                 + Clone {
        let grid = self.grid;
        self.range()
            .map(move |loc| unsafe { grid.get_unchecked(&loc) })
    }

    pub fn with_locations(
        &self,
    ) -> impl Iterator<Item = (Location, &'a G::Item)>
                 + DoubleEndedIterator
                 + FusedIterator
                 + ExactSizeIterator
                 + Debug
                 + Clone {
        let grid = self.grid;
        self.range()
            .map(move |loc| (loc, unsafe { grid.get_unchecked(&loc) }))
    }

    pub fn with_component(
        &self,
    ) -> impl Iterator<Item = (T::Converse, &'a G::Item)>
                 + DoubleEndedIterator
                 + FusedIterator
                 + ExactSizeIterator
                 + Debug
                 + Clone {
        let grid = self.grid;
        let index = self.index;
        self.grid.range().map(move |cross: T::Converse| {
            (cross, unsafe {
                grid.get_unchecked(&(cross.combine(index)))
            })
        })
    }
}

impl<'a, G: Grid + ?Sized, T: LocComponent> Index<T::Converse> for SingleView<'a, G, T> {
    type Output = G::Item;

    fn index(&self, idx: T::Converse) -> &G::Item {
        // TODO: insert error message once RangeError implements Error + Display
        self.get(idx)
            .unwrap_or_else(|_err| panic!("{} out of range", T::name()))
    }
}

// TODO: IntoIterator

pub type RowView<'a, G> = SingleView<'a, G, Row>;
pub type ColumnView<'a, G> = SingleView<'a, G, Column>;