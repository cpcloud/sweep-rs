use crate::error::Error;
use itertools::Itertools;
use std::collections::{HashMap, HashSet, VecDeque};

pub(crate) type Coordinate = (u16, u16);

#[derive(Debug, Clone)]
pub(crate) struct Tile {
    adjacent_tiles: HashSet<Coordinate>,
    pub(crate) mine: bool,
    pub(crate) exposed: bool,
    pub(crate) flagged: bool,
    pub(crate) adjacent_mines: u8,
}

#[derive(Debug, Copy, Clone, PartialEq)]
enum Increment {
    One,
    NegOne,
    Zero,
}

impl Increment {
    fn offset(&self, value: u16) -> u16 {
        match *self {
            Self::One => value + 1,
            Self::NegOne => value.saturating_sub(1),
            Self::Zero => value,
        }
    }
}

fn adjacent(
    (i, j): Coordinate,
    (nrows, ncolumns): (u16, u16),
) -> Result<HashSet<Coordinate>, Error> {
    let increments = [Increment::One, Increment::NegOne, Increment::Zero];

    Ok(increments
        .iter()
        .copied()
        .cartesian_product(increments.iter().copied())
        .filter_map(|(x, y)| {
            let x_offset = x.offset(i);
            let y_offset = y.offset(j);
            if (x != Increment::Zero || y != Increment::Zero)
                && x_offset < nrows
                && y_offset < ncolumns
            {
                Some((x_offset, y_offset))
            } else {
                None
            }
        })
        .collect())
}

pub(crate) struct Board {
    grid: HashMap<Coordinate, Tile>,
    pub(crate) nrows: u16,
    pub(crate) ncolumns: u16,
    nmines: u16,
    nflagged: u16,
}

fn index_from_coord((r, c): Coordinate, ncols: u16) -> usize {
    usize::from(r * ncols + c)
}

impl Board {
    pub(crate) fn new(nrows: u16, ncolumns: u16, nmines: u16) -> Result<Self, Error> {
        let mut rng = rand::thread_rng();
        let samples =
            rand::seq::index::sample(&mut rng, usize::from(nrows * ncolumns), usize::from(nmines))
                .into_iter()
                .collect::<HashSet<_>>();

        let grid = (0..nrows)
            .cartesian_product(0..ncolumns)
            .enumerate()
            .map(|(i, point)| {
                let adjacent_tiles = adjacent(point, (nrows, ncolumns))?;
                let adjacent_mines = adjacent_tiles.iter().fold(0, |total, &coord| {
                    total + u8::from(samples.contains(&index_from_coord(coord, ncolumns)))
                });
                assert!(adjacent_mines <= 8);

                Ok((
                    point,
                    Tile {
                        adjacent_tiles,
                        mine: samples.contains(&i),
                        exposed: false,
                        flagged: false,
                        adjacent_mines,
                    },
                ))
            })
            .collect::<Result<HashMap<_, _>, _>>()?;

        Ok(Self {
            nrows,
            ncolumns,
            grid,
            nmines,
            nflagged: 0,
        })
    }

    pub(crate) fn available_flags(&self) -> u16 {
        self.nmines - self.nflagged
    }

    pub(crate) fn win(&self) -> bool {
        let correctly_flagged_mines = self
            .grid
            .values()
            .map(|tile| u16::from(tile.flagged && tile.mine))
            .sum::<u16>();
        let total_exposed = self
            .grid
            .values()
            .map(|tile| u16::from(tile.exposed))
            .sum::<u16>();
        let exposed_or_correctly_flagged = total_exposed + correctly_flagged_mines;
        let ntiles = self.nrows * self.ncolumns;
        assert!(exposed_or_correctly_flagged <= ntiles);
        ntiles == exposed_or_correctly_flagged
    }

    pub(crate) fn expose(&mut self, i: u16, j: u16) -> Result<bool, Error> {
        if self.tile(i, j)?.mine {
            self.tile_mut(i, j)?.exposed = true;
            return Ok(true);
        }

        let mut seen = HashSet::new();
        let mut coordinates = [(i, j)].iter().copied().collect::<VecDeque<_>>();

        while let Some((x, y)) = coordinates.pop_front() {
            if seen.insert((x, y)) {
                let tile = self.tile_mut(x, y)?;

                tile.exposed = !(tile.mine || tile.flagged);

                if tile.adjacent_mines == 0 {
                    coordinates.extend(tile.adjacent_tiles.iter());
                }
            }
        }

        Ok(false)
    }

    pub(crate) fn expose_all(&mut self) -> Result<(), Error> {
        for (i, j) in self.grid.keys().copied().collect::<Vec<_>>().into_iter() {
            self.expose(i, j)?;
        }
        Ok(())
    }

    pub(crate) fn tile(&self, i: u16, j: u16) -> Result<&Tile, Error> {
        self.grid.get(&(i, j)).ok_or(Error::GetTile(i, j))
    }

    pub(crate) fn tile_mut(&mut self, i: u16, j: u16) -> Result<&mut Tile, Error> {
        self.grid.get_mut(&(i, j)).ok_or(Error::GetTile(i, j))
    }

    pub(crate) fn flag(&mut self, i: u16, j: u16) -> Result<bool, Error> {
        let nflagged = self.nflagged;
        let was_flagged = self.tile(i, j)?.flagged;
        let flagged = !was_flagged;
        let nmines = self.nmines;
        if was_flagged {
            self.nflagged = self.nflagged.saturating_sub(1);
            self.tile_mut(i, j)?.flagged = flagged;
        } else if nflagged < nmines && !self.tile(i, j)?.exposed {
            self.tile_mut(i, j)?.flagged = flagged;
            self.nflagged += 1;
        }
        Ok(flagged)
    }
}
