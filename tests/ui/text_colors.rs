use super::super::Preferences;
use super::*;

#[test]
fn menu_color_changes_leave_general_text_and_backgrounds_alone() {
    let base = Palette::default();
    let menu_only = TextColors {
        input_background: None,
        menu: Some([120, 130, 140]),
        general: None,
    }
    .apply(base);
    assert_eq!(menu_only.menu_text, egui::Color32::from_rgb(120, 130, 140));
    assert_eq!(menu_only.text, base.text);
    assert_eq!(menu_only.background, base.background);
    assert_eq!(menu_only.rail, base.rail);
    let general_only = TextColors {
        input_background: None,
        menu: None,
        general: Some([220, 230, 240]),
    }
    .apply(base);
    assert_eq!(general_only.menu_text, base.menu_text);
    assert_eq!(general_only.text, egui::Color32::from_rgb(220, 230, 240));
}

#[test]
fn color_choices_survive_restart_and_stay_separate_for_each_layout() {
    let mut prefs: Preferences = serde_json::from_str("{}").unwrap();
    assert!(prefs.text_colors.is_empty());
    prefs.text_colors.insert(
        "codex".into(),
        TextColors {
            input_background: Some([30, 40, 50]),
            menu: Some([100, 100, 100]),
            general: Some([230, 230, 230]),
        },
    );
    prefs.text_colors.insert(
        "gemini".into(),
        TextColors {
            input_background: None,
            menu: Some([150, 150, 150]),
            general: None,
        },
    );
    let restored: Preferences =
        serde_json::from_str(&serde_json::to_string(&prefs).unwrap()).unwrap();
    assert_eq!(restored.text_colors["codex"].menu, Some([100, 100, 100]));
    assert_eq!(
        restored.text_colors["codex"].input_background,
        Some([30, 40, 50])
    );
    let base = Palette::default();
    let custom = restored.text_colors["codex"].apply(base);
    assert_eq!(custom.input_background, egui::Color32::from_rgb(30, 40, 50));
    assert_eq!(custom.surface, base.surface);
    assert_eq!(restored.text_colors["gemini"].menu, Some([150, 150, 150]));
    assert!(restored.text_colors["gemini"].general.is_none());
    let base = Palette::default();
    assert_eq!(TextColors::default().apply(base).text, base.text);
}

#[test]
fn live_updates_change_default_text_without_changing_menu_text() {
    let context = egui::Context::default();
    let base = Palette::default();
    set_palette(base);
    apply(&context, "codex");
    let new = TextColors {
        input_background: None,
        general: Some([210, 220, 230]),
        menu: None,
    }
    .apply(base);
    update_colors(&context, new);
    assert_eq!(context.style().visuals.override_text_color, Some(new.text));
    assert_eq!(menu_text(), base.menu_text);
    set_palette(base);
}
