use gpui_kit::component::{Theme, ThemeMode};
use gpui_kit::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Appearance {
    Light,
    Dark,
    #[default]
    #[serde(other)]
    System,
}

impl Appearance {
    pub const ALL: [Self; 3] = [Self::System, Self::Light, Self::Dark];

    pub fn label(self) -> &'static str {
        match self {
            Self::System => tr!("Follow system"),
            Self::Light => tr!("Light"),
            Self::Dark => tr!("Dark"),
        }
    }

    pub fn mode(self, system: WindowAppearance) -> ThemeMode {
        match self {
            Self::System => system.into(),
            Self::Light => ThemeMode::Light,
            Self::Dark => ThemeMode::Dark,
        }
    }

    pub fn apply(self, window: &mut Window, cx: &mut App) {
        cx.set_window_appearance(match self {
            Self::System => None,
            Self::Light => Some(WindowAppearance::Light),
            Self::Dark => Some(WindowAppearance::Dark),
        });
        // Linux appearance notifications are provided by the window/desktop portal.
        let system = if cfg!(target_os = "macos") {
            cx.window_appearance()
        } else {
            window.appearance()
        };
        apply_mode(self.mode(system), cx);
    }
}

pub fn apply_mode(mode: ThemeMode, cx: &mut App) {
    Theme::change(mode, None, cx);
    Theme::update(cx, |theme| {
        theme.colors.primary = rgb(0x128252).into();
        theme.colors.primary_hover = rgb(0x0c7045).into();
        theme.colors.primary_active = rgb(0x095c38).into();
        theme.colors.primary_foreground = rgb(0xffffff).into();
        theme.colors.button_primary = rgb(0x128252).into();
        theme.colors.button_primary_hover = rgb(0x0c7045).into();
        theme.colors.button_primary_active = rgb(0x095c38).into();
        theme.colors.button_primary_foreground = rgb(0xffffff).into();
        theme.colors.ring = rgb(if mode.is_dark() { 0x63d4a0 } else { 0x128252 }).into();
        theme.font_size = px(13.);
        theme.mono_font_size = px(13.);
        theme.radius = px(6.);
        theme.shadow = false;
    });
}

// App surfaces retain the green identity; components and syntax use the same Theme mode.
pub fn color(light: u32, cx: &App) -> Rgba {
    rgb(if Theme::global(cx).mode.is_dark() {
        match light {
            0xffffff => 0x171d1a,
            0x25312d => 0xe5eee8,
            0x7b8580 => 0xa1b2a8,
            0xe4e9e6 | 0xf0f3f1 => 0x334139,
            0xf6f8f7 => 0x1e2722,
            0xeff3f1 | 0xf0f6f2 => 0x26372d,
            0xf5f8f6 | 0xeaf0ec | 0xe8efea => 0x2b3b31,
            0xe0f0e7 => 0x234b36,
            0x128252 | 0x096c42 => 0x63d4a0,
            0xc35246 => 0xff9589,
            _ => light,
        }
    } else {
        light
    })
}

#[cfg(test)]
mod tests {
    use super::{Appearance, ThemeMode, WindowAppearance};

    #[test]
    fn explicit_modes_ignore_system_and_system_tracks_both_appearances() {
        for system in [WindowAppearance::Light, WindowAppearance::Dark] {
            assert_eq!(Appearance::Light.mode(system), ThemeMode::Light);
            assert_eq!(Appearance::Dark.mode(system), ThemeMode::Dark);
            assert_eq!(Appearance::System.mode(system), ThemeMode::from(system));
        }
    }

    #[test]
    fn preferences_roundtrip_and_unknown_values_follow_system() {
        for appearance in Appearance::ALL {
            let json = serde_json::to_string(&appearance).unwrap();
            assert_eq!(
                serde_json::from_str::<Appearance>(&json).unwrap(),
                appearance
            );
        }
        assert_eq!(
            serde_json::from_str::<Appearance>("\"future\"").unwrap(),
            Appearance::System
        );
    }
}
