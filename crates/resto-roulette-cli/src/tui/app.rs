use ratatui::widgets::ListState;

use resto_roulette_core::bucket::{BucketEntry, Buckets};
use resto_roulette_core::picker::{self, Selection};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SortOrder {
    Name,
    TravelTime,
    Cuisine,
}

impl SortOrder {
    pub fn next(self) -> Self {
        match self {
            SortOrder::Name => SortOrder::TravelTime,
            SortOrder::TravelTime => SortOrder::Cuisine,
            SortOrder::Cuisine => SortOrder::Name,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            SortOrder::Name => "Name",
            SortOrder::TravelTime => "Time",
            SortOrder::Cuisine => "Cuisine",
        }
    }
}

pub struct BrowseState {
    pub bucket_idx: usize,
    pub cursor: usize,
    pub list_state: ListState,
}

impl BrowseState {
    pub fn new(bucket_idx: usize, initial_cursor: usize) -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(initial_cursor));
        Self {
            bucket_idx,
            cursor: initial_cursor,
            list_state,
        }
    }

    pub fn move_up(&mut self, len: usize) {
        if len == 0 {
            return;
        }
        self.cursor = self.cursor.saturating_sub(1);
        self.list_state.select(Some(self.cursor));
    }

    pub fn move_down(&mut self, len: usize) {
        if len == 0 {
            return;
        }
        self.cursor = (self.cursor + 1).min(len - 1);
        self.list_state.select(Some(self.cursor));
    }
}

pub struct ExplorerState {
    pub active_bucket: usize,
    pub cursor: usize,
    pub list_state: ListState,
    pub search_query: String,
    pub search_active: bool,
    pub sort_order: SortOrder,
    pub filtered_indices: Vec<usize>,
}

impl ExplorerState {
    pub fn new(active_bucket: usize, entries: &[BucketEntry]) -> Self {
        let mut state = Self {
            active_bucket,
            cursor: 0,
            list_state: ListState::default(),
            search_query: String::new(),
            search_active: false,
            sort_order: SortOrder::Name,
            filtered_indices: Vec::new(),
        };
        state.recompute_filtered(entries);
        state
    }

    pub fn recompute_filtered(&mut self, entries: &[BucketEntry]) {
        let q = self.search_query.to_lowercase();
        let mut indices: Vec<usize> = (0..entries.len())
            .filter(|&i| {
                if q.is_empty() {
                    return true;
                }
                let e = &entries[i];
                e.restaurant.name.to_lowercase().contains(&q)
                    || e.restaurant.address.to_lowercase().contains(&q)
                    || e.cuisines.iter().any(|c| c.to_lowercase().contains(&q))
            })
            .collect();

        let sort = self.sort_order;
        indices.sort_by(|&a, &b| match sort {
            SortOrder::Name => entries[a].restaurant.name.cmp(&entries[b].restaurant.name),
            SortOrder::TravelTime => entries[a].best_secs.cmp(&entries[b].best_secs),
            SortOrder::Cuisine => {
                // Entries with no recognized cuisine sort last
                let ca = entries[a].cuisines.first().map(String::as_str);
                let cb = entries[b].cuisines.first().map(String::as_str);
                match (ca, cb) {
                    (None, None) => entries[a].restaurant.name.cmp(&entries[b].restaurant.name),
                    (None, Some(_)) => std::cmp::Ordering::Greater,
                    (Some(_), None) => std::cmp::Ordering::Less,
                    (Some(x), Some(y)) => x.cmp(y),
                }
            }
        });

        self.filtered_indices = indices;
        if self.filtered_indices.is_empty() {
            self.cursor = 0;
            self.list_state.select(None);
        } else {
            self.cursor = self.cursor.min(self.filtered_indices.len() - 1);
            self.list_state.select(Some(self.cursor));
        }
    }

    pub fn move_up(&mut self) {
        if self.filtered_indices.is_empty() {
            return;
        }
        self.cursor = self.cursor.saturating_sub(1);
        self.list_state.select(Some(self.cursor));
    }

    pub fn move_down(&mut self) {
        let len = self.filtered_indices.len();
        if len == 0 {
            return;
        }
        self.cursor = (self.cursor + 1).min(len - 1);
        self.list_state.select(Some(self.cursor));
    }

    /// Returns the source index of the currently highlighted entry, if any.
    pub fn current_entry_idx(&self) -> Option<usize> {
        self.filtered_indices.get(self.cursor).copied()
    }
}

pub enum Action {
    Nothing,
    SwitchToSlots,
    SwitchToBrowse(BrowseState),
    SwitchToExplorer(ExplorerState),
    Accept,
    Quit,
}

#[derive(Clone, Copy, PartialEq)]
pub enum ModeKind {
    Slots,
    Browse,
    Explorer,
}

pub enum Mode {
    Slots,
    Browse(BrowseState),
    Explorer(ExplorerState),
}

pub struct App<'a> {
    pub buckets: &'a Buckets,
    pub slots: [Option<BucketEntry>; 3],
    pub selected: usize,
    pub mode: Mode,
}

impl<'a> App<'a> {
    pub fn new(buckets: &'a Buckets, initial: Selection) -> Self {
        Self {
            buckets,
            slots: [initial.near, initial.mid, initial.far],
            selected: 0,
            mode: Mode::Slots,
        }
    }

    pub fn new_in_explorer(buckets: &'a Buckets, initial: Selection) -> Self {
        let slots = [initial.near, initial.mid, initial.far];
        let active_bucket = 0usize;
        let entries = match active_bucket {
            0 => buckets.near.as_slice(),
            1 => buckets.mid.as_slice(),
            _ => buckets.far.as_slice(),
        };
        let explorer_state = ExplorerState::new(active_bucket, entries);
        Self {
            buckets,
            slots,
            selected: 0,
            mode: Mode::Explorer(explorer_state),
        }
    }

    pub fn mode_kind(&self) -> ModeKind {
        match &self.mode {
            Mode::Slots => ModeKind::Slots,
            Mode::Browse(_) => ModeKind::Browse,
            Mode::Explorer(_) => ModeKind::Explorer,
        }
    }

    pub fn move_up(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    pub fn move_down(&mut self) {
        self.selected = (self.selected + 1).min(2);
    }

    pub fn reroll_selected(&mut self) {
        let candidates = match self.selected {
            0 => &self.buckets.near,
            1 => &self.buckets.mid,
            _ => &self.buckets.far,
        };
        self.slots[self.selected] = picker::pick_one_random(candidates);
    }

    pub fn reroll_all(&mut self) {
        self.slots[0] = picker::pick_one_random(&self.buckets.near);
        self.slots[1] = picker::pick_one_random(&self.buckets.mid);
        self.slots[2] = picker::pick_one_random(&self.buckets.far);
    }

    pub fn to_selection(&self) -> Selection {
        Selection {
            near: self.slots[0].clone(),
            mid: self.slots[1].clone(),
            far: self.slots[2].clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use resto_roulette_core::bucket::Bucket;
    use resto_roulette_core::routing::models::TravelMode;
    use resto_roulette_core::Restaurant;

    fn make_entry(name: &str, secs: u32, cuisine: Option<&str>) -> BucketEntry {
        BucketEntry {
            restaurant: Restaurant {
                name: name.into(),
                address: format!("{} St", name),
                location: None,
            },
            bucket: Bucket::Near,
            best_secs: secs,
            best_mode: TravelMode::Transit,
            cuisines: cuisine.map(|c| vec![c.to_string()]).unwrap_or_default(),
        }
    }

    #[test]
    fn browse_state_cursor_clamps_at_zero() {
        let mut state = BrowseState::new(0, 0);
        state.move_up(3);
        assert_eq!(state.cursor, 0);
    }

    #[test]
    fn browse_state_cursor_clamps_at_max() {
        let mut state = BrowseState::new(0, 0);
        state.move_down(3);
        state.move_down(3);
        state.move_down(3);
        assert_eq!(state.cursor, 2);
    }

    #[test]
    fn browse_state_noop_when_empty() {
        let mut state = BrowseState::new(0, 0);
        state.move_up(0);
        state.move_down(0);
        assert_eq!(state.cursor, 0);
    }

    #[test]
    fn explorer_state_search_filters_by_name() {
        let entries = vec![
            make_entry("Hà", 720, Some("vietnamese")),
            make_entry("Nouveau Palais", 1200, Some("diner")),
            make_entry("Joe Beef", 2400, None),
        ];
        let mut state = ExplorerState::new(0, &entries);
        state.search_query = "nouveau".into();
        state.recompute_filtered(&entries);
        assert_eq!(state.filtered_indices.len(), 1);
        assert_eq!(
            entries[state.filtered_indices[0]].restaurant.name,
            "Nouveau Palais"
        );
    }

    #[test]
    fn explorer_state_search_filters_by_cuisine() {
        let entries = vec![
            make_entry("Hà", 720, Some("vietnamese")),
            make_entry("Ikemen", 800, Some("ramen")),
            make_entry("Burger Joint", 900, Some("burger")),
        ];
        let mut state = ExplorerState::new(0, &entries);
        state.search_query = "ramen".into();
        state.recompute_filtered(&entries);
        assert_eq!(state.filtered_indices.len(), 1);
        assert_eq!(entries[state.filtered_indices[0]].restaurant.name, "Ikemen");
    }

    #[test]
    fn explorer_state_sort_by_name() {
        let entries = vec![
            make_entry("Zebra", 720, None),
            make_entry("Apple", 800, None),
            make_entry("Mango", 900, None),
        ];
        let state = ExplorerState::new(0, &entries);
        // Default sort is Name
        let names: Vec<_> = state
            .filtered_indices
            .iter()
            .map(|&i| entries[i].restaurant.name.as_str())
            .collect();
        assert_eq!(names, vec!["Apple", "Mango", "Zebra"]);
    }

    #[test]
    fn explorer_state_sort_by_time() {
        let entries = vec![
            make_entry("Slow", 1800, None),
            make_entry("Fast", 300, None),
            make_entry("Medium", 900, None),
        ];
        let mut state = ExplorerState::new(0, &entries);
        state.sort_order = SortOrder::TravelTime;
        state.recompute_filtered(&entries);
        let names: Vec<_> = state
            .filtered_indices
            .iter()
            .map(|&i| entries[i].restaurant.name.as_str())
            .collect();
        assert_eq!(names, vec!["Fast", "Medium", "Slow"]);
    }

    #[test]
    fn explorer_state_sort_by_cuisine_no_cuisine_last() {
        let entries = vec![
            make_entry("NoCuisine", 720, None),
            make_entry("ZenBurger", 800, Some("burger")),
            make_entry("Pho Place", 900, Some("vietnamese")),
        ];
        let mut state = ExplorerState::new(0, &entries);
        state.sort_order = SortOrder::Cuisine;
        state.recompute_filtered(&entries);
        let last_idx = *state.filtered_indices.last().unwrap();
        assert_eq!(entries[last_idx].restaurant.name, "NoCuisine");
    }

    #[test]
    fn explorer_state_cursor_clamped_after_filter_reduces_results() {
        let entries = vec![
            make_entry("Alpha", 720, None),
            make_entry("Beta", 800, None),
            make_entry("Gamma", 900, None),
        ];
        let mut state = ExplorerState::new(0, &entries);
        state.cursor = 2;
        state.list_state.select(Some(2));
        state.search_query = "alpha".into();
        state.recompute_filtered(&entries);
        assert_eq!(state.cursor, 0); // clamped to 0 since only 1 result
    }

    #[test]
    fn explorer_state_empty_entries() {
        let state = ExplorerState::new(0, &[]);
        assert_eq!(state.filtered_indices.len(), 0);
        assert_eq!(state.cursor, 0);
        assert_eq!(state.list_state.selected(), None);
    }
}
