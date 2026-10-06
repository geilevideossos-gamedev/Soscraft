use bevy::prelude::*;
use frame::ClientSet;

use crate::classes::store::{
    ClassStoreFile, load_class_store, save_class_store, sync_host_class_loadouts,
};

#[derive(Resource, Clone, Debug, Default)]
pub struct MenuMapList(pub Vec<asset_transport::MapPack>);

impl MenuMapList {
    pub fn maps(&self) -> impl Iterator<Item = &String> {
        self.0.iter().flat_map(|pack| &pack.maps)
    }

    pub fn contains(&self, map: &str) -> bool {
        self.maps().any(|installed| installed == map)
    }

    pub fn pack_of(&self, map: &str) -> Option<usize> {
        self.0
            .iter()
            .position(|pack| pack.maps.iter().any(|installed| installed == map))
    }
}

pub(crate) struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MenuMapList>()
            .init_resource::<crate::ClassLoadoutCatalog>()
            .init_resource::<frame::GameSettings>()
            .init_resource::<crate::BindingView>()
            .init_resource::<crate::SessionClassStore>()
            .init_resource::<ClassStoreFile>()
            .init_resource::<frame::HostClassLoadouts>()
            .init_resource::<asset_game::MenuCatalog>()
            .init_resource::<asset_game::LocalizeCatalog>()
            .add_systems(
                Update,
                (
                    crate::options::apply_window_settings,
                    load_class_store,
                    sync_host_class_loadouts,
                    save_class_store,
                )
                    .chain()
                    .in_set(ClientSet::Ui),
            );
    }
}

pub fn install_frontend_menus(catalog: &mut asset_game::MenuCatalog) -> Result<(), String> {
    catalog.load_definitions(include_str!("../menus/frontend.json"))?;
    catalog.load_definitions(include_str!("../menus/connection_error.json"))?;
    catalog.load_definitions(include_str!("../menus/classes.json"))?;
    catalog.load_definitions(include_str!("../menus/settings.json"))?;
    catalog.load_definitions(include_str!("../menus/controller.json"))?;
    let has_controller_page = catalog.get("options_controller").is_some();
    for (name, menu) in &mut catalog.menus {
        if matches!(name.as_str(), "popup_endgame" | "popup_endgame_ranked") {
            for item in &mut menu.items {
                if item.name == "button_yes" {
                    item.handlers.action = vec![asset_game::MenuEvent::Script(
                        "play mouse_click; close self; exec \"disconnect\";".into(),
                    )];
                }
            }
        }
        if let Some(settings_link) = menu
            .items
            .iter()
            .find(|item| {
                item.item_type == 1
                    && matches!(item.text_key.as_str(), "@MENU_CHAT" | "@MENU_VOICE")
            })
            .cloned()
            && menu
                .items
                .iter()
                .any(|item| item.text_key == "@MENU_RESET_SYSTEM_DEFAULTS")
        {
            let mut multiplayer = settings_link;
            multiplayer.name = "multiplayer_settings".into();
            multiplayer.text_key = "@MENU_MULTIPLAYER_OPTIONS".into();
            multiplayer.rect.y = 88.0;
            multiplayer.vis_exp = "1".into();
            multiplayer.disabled_exp = "0".into();
            multiplayer.handlers.action = vec![asset_game::MenuEvent::Script(
                "play mouse_click; close self; open options_multi;".into(),
            )];
            let mut controller = multiplayer.clone();
            menu.items.push(multiplayer);
            if has_controller_page {
                controller.name = "controller_settings".into();
                controller.text_key = "Controller".into();
                controller.rect.y = 108.0;
                controller.handlers.action = vec![asset_game::MenuEvent::Script(
                    "play mouse_click; close self; open options_controller;".into(),
                )];
                menu.items.push(controller);
            }
        }

        let removed_rows: Vec<_> = menu
            .items
            .iter()
            .filter(|item| {
                item.item_type == 1
                    && matches!(
                        item.text_key.as_str(),
                        "@MENU_VOICE" | "@MENU_CHAT" | "@MENU_RESET_SYSTEM_DEFAULTS"
                    )
            })
            .map(|item| (item.rect.x, item.rect.y))
            .collect();
        menu.items.retain(|item| {
            !removed_rows.iter().any(|&(x, y)| {
                item.rect.x == x
                    && item.rect.y == y
                    && item.name != "multiplayer_settings"
                    && item.name != "controller_settings"
            })
        });
        if name == "pc_options_controls" {
            for item in &mut menu.items {
                if item.rect.x >= 232.0 && item.rect.y > 88.0 {
                    item.rect.y -= 20.0;
                }
            }
        }

        let Some(mode) = name.strip_prefix("settings_quick_") else {
            continue;
        };
        if sim::HostGameModeSelection::from_token(mode).is_none() {
            continue;
        }
        for item in &mut menu.items {
            if item.dvar == "camera_thirdperson" {
                if item.item_type == 12 {
                    item.choices.clear();
                    item.dvar.clear();
                    item.text_key = "Недоступно".into();
                }
                item.disabled_exp = "1".into();
                item.item_type = 0;
                item.static_flags |= 0x100000;
                item.handlers.action.clear();
                item.fore_color[3] = 0.4;
            } else if item.item_type == 1 && !item.dvar.is_empty() {
                item.text_scale = 0.30;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod settings_menu_tests {
    use asset_game::{MenuCatalog, MenuDef, MenuEvent, MenuItem};

    fn item(name: &str, text_key: &str, dvar: &str, item_type: i32) -> MenuItem {
        MenuItem {
            name: name.into(),
            text_key: text_key.into(),
            dvar: dvar.into(),
            item_type,
            vis_exp: "1".into(),
            ..Default::default()
        }
    }

    fn base(name: &str, items: Vec<MenuItem>) -> (String, MenuDef) {
        let mut menu = MenuDef::default();
        menu.name = name.into();
        menu.items = items;
        (name.into(), menu)
    }

    fn settings_catalog() -> MenuCatalog {
        let mut catalog = MenuCatalog::default();
        catalog.menus.extend([
            base(
                "pc_options_video",
                vec![
                    item("", "@MENU_VIDEO", "", 0),
                    item("", "", "ui_r_mode", 12),
                    item("", "@MENU_VIDEO_MODE", "", 0),
                    item("", "@PLATFORM_BACK_CAPS", "", 1),
                    item("", "", "profileMenuOption_Gamma", 10),
                    item("", "@MENU_BRIGHTNESS", "", 0),
                    item("", "@MENU_ADVANCED_VIDEO", "", 1),
                ],
            ),
            base(
                "pc_options_audio",
                vec![
                    item("", "", "profileMenuOption_volume", 10),
                    item("", "", "ui_outputConfig", 12),
                    item("", "@MENU_SPEAKER_CONFIG", "", 0),
                ],
            ),
            base(
                "options_multi",
                vec![
                    item("", "", "ui_name", 4),
                    item("", "@MENU_PLAYER_NAME", "", 0),
                ],
            ),
        ]);
        catalog
            .load_definitions(include_str!("../menus/settings.json"))
            .expect("settings menus load over their bases");
        catalog
    }

    fn script(events: &[MenuEvent]) -> String {
        events
            .iter()
            .filter_map(|event| match event {
                MenuEvent::Script(text) => Some(text.as_str()),
                _ => None,
            })
            .collect()
    }

    fn named<'a>(menu: &'a MenuDef, name: &str) -> &'a MenuItem {
        menu.items
            .iter()
            .find(|item| item.name == name)
            .unwrap_or_else(|| panic!("{} has no item {name}", menu.name))
    }

    #[test]
    fn advanced_video_replaces_the_video_page_instead_of_stacking_on_it() {
        let catalog = settings_catalog();
        let video = catalog.get("pc_options_video").expect("video page");
        let action = script(&named(video, "video_advanced").handlers.action);
        let close = action.find("close self;").expect("video page closes");
        let open = action
            .find("open pc_options_advanced_video;")
            .expect("advanced video opens");
        assert!(close < open, "{action}");
    }

    #[test]
    fn esc_and_back_on_advanced_video_return_to_the_video_page() {
        let catalog = settings_catalog();
        let advanced = catalog
            .get("pc_options_advanced_video")
            .expect("advanced video page");
        assert_eq!(
            script(&advanced.handlers.esc),
            "close self; open pc_options_video;"
        );
        let back = advanced
            .items
            .iter()
            .find(|item| item.text_key == "@PLATFORM_BACK_CAPS")
            .expect("back button");
        assert_eq!(
            script(&back.handlers.action),
            "close self; open pc_options_video;"
        );
        let video = catalog.get("pc_options_video").expect("video page");
        assert_eq!(script(&video.handlers.esc), "close self;");
    }

    #[test]
    fn advanced_video_offers_a_show_blood_choice() {
        let catalog = settings_catalog();
        let advanced = catalog
            .get("pc_options_advanced_video")
            .expect("advanced video page");
        let row = named(advanced, "ui_show_blood");
        assert_eq!(row.dvar, "ui_show_blood");
        assert_eq!(row.item_type, 12);
        assert_eq!(row.vis_exp, "1");
        assert_eq!(
            row.choices,
            vec![
                ("@MENU_DISABLED".to_owned(), "0".to_owned()),
                ("@MENU_ENABLED".to_owned(), "1".to_owned())
            ]
        );
        let label = named(advanced, "advanced_label_11");
        assert_eq!(label.text_key, "Show Blood");
        assert_eq!(label.rect.y, row.rect.y);
        assert!(
            advanced
                .items
                .iter()
                .filter(|item| item.vis_exp != "0" && item.name != "advanced_label_11")
                .all(|item| item.name == "ui_show_blood" || item.rect.y != row.rect.y),
            "another visible row shares the Show Blood line"
        );
    }
}
