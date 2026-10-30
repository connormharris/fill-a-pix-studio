use crate::error::{CoreError, Result};
use crate::grid::{Grid, Pos};
use std::collections::{BTreeMap, VecDeque};

pub const MIN_LEVEL: u8 = 1;
pub const MAX_LEVEL: u8 = 4;
pub const MAX_MARGIN: u8 = 2;
pub const DEFAULT_MARGIN: u8 = 2;
pub const DEFAULT_LEVEL: u8 = 2;
/// Largest region ID accepted from files; keeps `next_id` arithmetic far from overflow.
pub const MAX_REGION_ID: u32 = u32::MAX / 2;

/// Identifier of a region; positive in files, unique within a `Regions`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RegionId(pub u32);

/// Per-region generation settings.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RegionSettings {
    pub level: u8,
    pub margin: u8,
    pub name: Option<String>,
}

/// A partition of (part of) the grid into regions. Cells may belong to no region.
///
/// Equality compares the map, table and default level only. `next_id` is a
/// non-persisted allocation cursor (files never store it), so it is ignored.
#[derive(Clone, Debug)]
pub struct Regions {
    map: Grid<Option<RegionId>>,
    table: BTreeMap<RegionId, RegionSettings>,
    default_level: u8,
    next_id: u32,
}

impl PartialEq for Regions {
    fn eq(&self, other: &Regions) -> bool {
        self.map == other.map
            && self.table == other.table
            && self.default_level == other.default_level
    }
}

impl Eq for Regions {}

fn check_level(level: u8, what: &str) -> Result<()> {
    if (MIN_LEVEL..=MAX_LEVEL).contains(&level) {
        Ok(())
    } else {
        Err(CoreError::Format(format!(
            "{what} level {level} is out of range ({MIN_LEVEL}-{MAX_LEVEL})"
        )))
    }
}

fn check_margin(margin: u8, what: &str) -> Result<()> {
    if margin <= MAX_MARGIN {
        Ok(())
    } else {
        Err(CoreError::Format(format!(
            "{what} margin {margin} is out of range (0-{MAX_MARGIN})"
        )))
    }
}

impl Regions {
    /// No regions. Panics if a dimension is 0.
    pub fn new(width: usize, height: usize, default_level: u8) -> Regions {
        Regions {
            map: Grid::new(width, height, None),
            table: BTreeMap::new(),
            default_level,
            next_id: 1,
        }
    }

    /// Validates and normalizes. `next_id` starts at (largest ID in table or map) + 1,
    /// or 1 when there are none, before the split rule runs.
    pub fn from_parts(
        map: Grid<Option<RegionId>>,
        table: BTreeMap<RegionId, RegionSettings>,
        default_level: u8,
    ) -> Result<Regions> {
        Self::check_parts(&map, &table, default_level)?;
        let max_map = map
            .positions()
            .filter_map(|p| *map.get(p))
            .map(|i| i.0)
            .max();
        let max_table = table.keys().next_back().map(|i| i.0);
        let next_id = max_map
            .max(max_table)
            .map_or(1, |m| m.checked_add(1).expect("region id too large"));
        let mut r = Regions {
            map,
            table,
            default_level,
            next_id,
        };
        r.normalize();
        Ok(r)
    }

    fn check_parts(
        map: &Grid<Option<RegionId>>,
        table: &BTreeMap<RegionId, RegionSettings>,
        default_level: u8,
    ) -> Result<()> {
        check_level(default_level, "default")?;
        for (id, s) in table {
            if id.0 > MAX_REGION_ID {
                return Err(CoreError::Format(format!(
                    "region id {} is too large",
                    id.0
                )));
            }
            check_level(s.level, &format!("region {}", id.0))?;
            check_margin(s.margin, &format!("region {}", id.0))?;
        }
        for p in map.positions() {
            if let Some(id) = *map.get(p) {
                if id.0 > MAX_REGION_ID {
                    return Err(CoreError::Format(format!(
                        "region id {} is too large",
                        id.0
                    )));
                }
                if !table.contains_key(&id) {
                    return Err(CoreError::Format(format!(
                        "region {} at {} is not defined",
                        id.0,
                        p.describe()
                    )));
                }
            }
        }
        Ok(())
    }

    /// Checks levels (including the default, 1-4), margins (0-2) and that every map ID
    /// is in the table. Mutators do not validate; writers call this.
    pub fn validate(&self) -> Result<()> {
        Self::check_parts(&self.map, &self.table, self.default_level)
    }

    fn alloc_id(&mut self) -> RegionId {
        let id = RegionId(self.next_id);
        self.next_id = self.next_id.checked_add(1).expect("region id overflow");
        id
    }

    pub fn map(&self) -> &Grid<Option<RegionId>> {
        &self.map
    }

    pub fn table(&self) -> &BTreeMap<RegionId, RegionSettings> {
        &self.table
    }

    pub fn default_level(&self) -> u8 {
        self.default_level
    }

    pub fn set_default_level(&mut self, l: u8) {
        self.default_level = l;
    }

    pub fn width(&self) -> usize {
        self.map.width()
    }

    pub fn height(&self) -> usize {
        self.map.height()
    }

    pub fn region_at(&self, p: Pos) -> Option<RegionId> {
        *self.map.get(p)
    }

    pub fn settings(&self, id: RegionId) -> Option<&RegionSettings> {
        self.table.get(&id)
    }

    pub fn settings_mut(&mut self, id: RegionId) -> Option<&mut RegionSettings> {
        self.table.get_mut(&id)
    }

    /// Cells of a region in row-major order.
    pub fn cells(&self, id: RegionId) -> Vec<Pos> {
        self.map
            .positions()
            .filter(|&p| *self.map.get(p) == Some(id))
            .collect()
    }

    pub fn default_settings(&self) -> RegionSettings {
        RegionSettings {
            level: self.default_level,
            margin: DEFAULT_MARGIN,
            name: None,
        }
    }

    /// Adds a region with a fresh ID and no cells.
    pub fn create_region(&mut self, s: RegionSettings) -> RegionId {
        let id = self.alloc_id();
        self.table.insert(id, s);
        id
    }

    /// Raw write. Panics if `id` is not in the table. Run `normalize()` after a batch.
    pub fn assign(&mut self, p: Pos, id: Option<RegionId>) {
        if let Some(id) = id {
            assert!(self.table.contains_key(&id), "unknown region {}", id.0);
        }
        self.map.set(p, id);
    }

    /// Applies the split rule: every region must be 4-connected. A disconnected region's
    /// ID is removed and each piece gets a fresh ID (allocated in row-major order of the
    /// piece's first cell) with a clone of the settings. Cell-less table entries are dropped.
    pub fn normalize(&mut self) {
        let (w, h) = (self.width(), self.height());
        let mut visited = Grid::new(w, h, false);
        // Components in row-major order of first cell.
        let mut comps: Vec<(RegionId, Vec<Pos>)> = Vec::new();
        for start in self.map.positions() {
            let Some(id) = *self.map.get(start) else {
                continue;
            };
            if *visited.get(start) {
                continue;
            }
            visited.set(start, true);
            let mut cells = Vec::new();
            let mut queue = VecDeque::from([start]);
            while let Some(p) = queue.pop_front() {
                cells.push(p);
                let (r, c) = (p.row as isize, p.col as isize);
                for (dr, dc) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
                    if let Some(q) = self.map.pos_at(r + dr, c + dc)
                        && !*visited.get(q)
                        && *self.map.get(q) == Some(id)
                    {
                        visited.set(q, true);
                        queue.push_back(q);
                    }
                }
            }
            comps.push((id, cells));
        }
        let mut counts: BTreeMap<RegionId, usize> = BTreeMap::new();
        for (id, _) in &comps {
            *counts.entry(*id).or_default() += 1;
        }
        let mut new_table = BTreeMap::new();
        for (id, cells) in comps {
            if counts[&id] == 1 {
                new_table.insert(id, self.table[&id].clone());
                continue;
            }
            let new_id = self.alloc_id();
            new_table.insert(new_id, self.table[&id].clone());
            for p in cells {
                self.map.set(p, Some(new_id));
            }
        }
        self.table = new_table;
    }

    /// Vertical strips preset. `k = max(1, width / strip_width)` strips with IDs `1..=k`
    /// left to right; the last absorbs leftover columns. Levels interpolate from
    /// `start_level` to `end_level`. Ends with `next_id = k + 1`.
    ///
    /// Panics if `strip_width == 0`, if any level is outside 1-4, or if `margin > 2`.
    pub fn strips(
        width: usize,
        height: usize,
        strip_width: usize,
        start_level: u8,
        end_level: u8,
        margin: u8,
        default_level: u8,
    ) -> Regions {
        assert!(strip_width > 0, "strip_width must be non-zero");
        for (l, what) in [
            (start_level, "start"),
            (end_level, "end"),
            (default_level, "default"),
        ] {
            check_level(l, what).unwrap_or_else(|e| panic!("{e}"));
        }
        check_margin(margin, "strip").unwrap_or_else(|e| panic!("{e}"));
        let k = (width / strip_width).max(1);
        let mut r = Regions::new(width, height, default_level);
        for i in 0..k {
            let level = if k == 1 {
                start_level
            } else {
                let (s, e) = (f64::from(start_level), f64::from(end_level));
                (s + (e - s) * i as f64 / (k - 1) as f64).round() as u8
            };
            let id = r.create_region(RegionSettings {
                level,
                margin,
                name: None,
            });
            let c0 = i * strip_width;
            let c1 = if i + 1 == k {
                width
            } else {
                (i + 1) * strip_width
            };
            for row in 0..height {
                for col in c0..c1 {
                    r.map.set(Pos::new(row, col), Some(id));
                }
            }
        }
        r
    }
}
