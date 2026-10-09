use crate::McStack;

pub const MC_CREATIVE_COLUMNS: usize = 9;
pub const MC_CREATIVE_ROWS: usize = 5;
pub const MC_SEARCH_LIMIT: usize = 50;
pub const MC_HOTBAR_GROUPS: usize = 9;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum McTabKind {
    #[default]
    Category,
    Hotbar,
    Search,
    Inventory,
}

#[derive(Clone, Debug, PartialEq)]
pub struct McCreativeTab {
    pub id: String,
    pub title: String,
    pub top: bool,
    pub column: u8,
    pub aligned_right: bool,
    pub kind: McTabKind,
    pub show_title: bool,
    pub scroll_bar: bool,
    pub background: String,
    pub icon: Option<McStack>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct McCreative {
    pub tabs: Vec<McCreativeTab>,
    pub selected: Option<String>,
    pub query: String,
    pub cursor: usize,
    pub highlight: usize,
    pub scroll: f32,
    pub items: Vec<Option<McStack>>,
    pub tags: Vec<Vec<String>>,
    pub tabs_of: std::collections::HashMap<String, Vec<String>>,
    pub hotbar_key: Option<(usize, bool)>,
    pub saved: Option<String>,
}

impl McCreative {
    pub fn selected_index(&self) -> Option<usize> {
        let kept = self
            .selected
            .as_ref()
            .and_then(|id| self.tabs.iter().position(|tab| &tab.id == id));
        kept.or_else(|| self.default_index())
    }

    pub fn default_index(&self) -> Option<usize> {
        self.tabs
            .iter()
            .position(|tab| tab.kind == McTabKind::Category)
            .or_else(|| (!self.tabs.is_empty()).then_some(0))
    }

    pub fn selected_tab(&self) -> Option<&McCreativeTab> {
        self.tabs.get(self.selected_index()?)
    }

    pub fn first_shown(&self) -> usize {
        first_row(self.scroll, self.items.len()) * MC_CREATIVE_COLUMNS
    }
}

pub fn hidden_rows(items: usize) -> usize {
    items
        .div_ceil(MC_CREATIVE_COLUMNS)
        .saturating_sub(MC_CREATIVE_ROWS)
}

pub fn can_scroll(items: usize) -> bool {
    items > MC_CREATIVE_COLUMNS * MC_CREATIVE_ROWS
}

pub fn first_row(scroll: f32, items: usize) -> usize {
    (scroll * hidden_rows(items) as f32 + 0.5).max(0.0) as usize
}

pub fn scroll_after_wheel(scroll: f32, notches: f32, items: usize) -> f32 {
    let hidden = hidden_rows(items);
    if hidden == 0 {
        return 0.0;
    }
    (scroll - notches / hidden as f32).clamp(0.0, 1.0)
}

pub fn scroll_for_drag(mouse_y: f64) -> f32 {
    ((mouse_y as f32 - 18.0 - 7.5) / (112.0 - 15.0)).clamp(0.0, 1.0)
}

pub fn scroller_y(scroll: f32) -> i32 {
    18 + ((112 - 17) as f32 * scroll) as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tab(id: &str, kind: McTabKind) -> McCreativeTab {
        McCreativeTab {
            id: id.to_owned(),
            title: id.to_owned(),
            top: true,
            column: 0,
            aligned_right: false,
            kind,
            show_title: true,
            scroll_bar: true,
            background: "items".to_owned(),
            icon: None,
        }
    }

    #[test]
    fn the_kept_tab_wins_and_a_missing_one_falls_back_to_the_first_category() {
        let mut creative = McCreative {
            tabs: vec![
                tab("minecraft:hotbar", McTabKind::Hotbar),
                tab("minecraft:building_blocks", McTabKind::Category),
                tab("minecraft:search", McTabKind::Search),
            ],
            ..McCreative::default()
        };
        assert_eq!(creative.selected_index(), Some(1));
        creative.selected = Some("minecraft:search".to_owned());
        assert_eq!(creative.selected_index(), Some(2));
        creative.selected = Some("minecraft:op_blocks".to_owned());
        assert_eq!(creative.selected_index(), Some(1));
        assert_eq!(McCreative::default().selected_index(), None);
    }

    #[test]
    fn scrolling_moves_by_whole_rows_like_vanilla() {
        assert_eq!(hidden_rows(45), 0);
        assert_eq!(hidden_rows(46), 1);
        assert_eq!(hidden_rows(475), 48);
        assert!(!can_scroll(45) && can_scroll(46));
        assert_eq!(first_row(0.0, 475), 0);
        assert_eq!(first_row(1.0, 475), 48);
        assert_eq!(first_row(0.51 / 48.0, 475), 1);
        assert_eq!(first_row(0.49 / 48.0, 475), 0);
        assert_eq!(scroll_after_wheel(0.0, -1.0, 475), 1.0 / 48.0);
        assert_eq!(scroll_after_wheel(0.0, 1.0, 475), 0.0);
        assert_eq!(scroll_after_wheel(0.5, 3.0, 45), 0.0);
        assert_eq!(scroll_for_drag(18.0 + 7.5), 0.0);
        assert_eq!(scroll_for_drag(18.0 + 7.5 + 97.0), 1.0);
        assert_eq!(scroller_y(0.0), 18);
        assert_eq!(scroller_y(1.0), 113);
        let creative = McCreative {
            items: vec![None; 475],
            scroll: 1.0,
            ..McCreative::default()
        };
        assert_eq!(creative.first_shown(), 432);
    }
}
