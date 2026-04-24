use std::cell::RefCell;

use glib::Properties;

mod imp {
    use super::*;
    use gdk4::prelude::ObjectExt;
    use glib::subclass::prelude::*;

    #[derive(Debug, Default, Properties)]
    #[properties(wrapper_type = super::MenuEntry)]
    pub struct MenuEntry {
        #[property(get, set)]
        pub label: RefCell<String>,
        #[property(get, set)]
        pub value: RefCell<Option<String>>,
        #[property(get, set)]
        pub icon: RefCell<Option<String>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MenuEntry {
        const NAME: &'static str = "MenuEntry";
        type Type = super::MenuEntry;
    }

    #[glib::derived_properties]
    impl ObjectImpl for MenuEntry {}
}

glib::wrapper! {
    pub struct MenuEntry(ObjectSubclass<imp::MenuEntry>);
}

impl MenuEntry {
    pub fn new(label: &str, icon: Option<&str>, value: Option<&str>) -> Self {
        glib::Object::builder()
            .property("label", label)
            .property("icon", icon.map(|s| s.to_string()))
            .property("value", value.map(|s| s.to_string()))
            .build()
    }
}

pub fn parse_line(line: &str) -> Option<(&str, Option<&str>, Option<&str>)> {
    let parts: Vec<&str> = line.splitn(3, ',').collect();
    let label = parts[0].trim();
    if label.is_empty() {
        return None;
    }
    let icon = parts.get(1).and_then(|s| non_empty_trimmed(s));
    let value = parts.get(2).and_then(|s| non_empty_trimmed(s));
    Some((label, icon, value))
}

fn non_empty_trimmed(s: &str) -> Option<&str> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_empty_trimmed_strips_whitespace() {
        assert_eq!(non_empty_trimmed("  foo  "), Some("foo"));
        assert_eq!(non_empty_trimmed("bar"), Some("bar"));
    }

    #[test]
    fn non_empty_trimmed_returns_none_for_empty_or_whitespace() {
        assert_eq!(non_empty_trimmed(""), None);
        assert_eq!(non_empty_trimmed("   "), None);
        assert_eq!(non_empty_trimmed("\t\n"), None);
    }

    #[test]
    fn parse_line_label_only() {
        assert_eq!(parse_line("Firefox"), Some(("Firefox", None, None)));
    }

    #[test]
    fn parse_line_label_and_icon() {
        assert_eq!(
            parse_line("Firefox,firefox.png"),
            Some(("Firefox", Some("firefox.png"), None))
        );
    }

    #[test]
    fn parse_line_all_three_fields() {
        assert_eq!(
            parse_line("Firefox,firefox.png,firefox --new-window"),
            Some(("Firefox", Some("firefox.png"), Some("firefox --new-window")))
        );
    }

    #[test]
    fn parse_line_trims_each_field() {
        assert_eq!(
            parse_line("  Firefox , firefox.png , cmd "),
            Some(("Firefox", Some("firefox.png"), Some("cmd")))
        );
    }

    #[test]
    fn parse_line_empty_label_returns_none() {
        assert_eq!(parse_line(""), None);
        assert_eq!(parse_line("   "), None);
        assert_eq!(parse_line(",icon,value"), None);
    }

    #[test]
    fn parse_line_empty_icon_or_value_becomes_none() {
        assert_eq!(
            parse_line("Firefox,,cmd"),
            Some(("Firefox", None, Some("cmd")))
        );
        assert_eq!(
            parse_line("Firefox,icon,"),
            Some(("Firefox", Some("icon"), None))
        );
    }

    #[test]
    fn parse_line_does_not_split_beyond_three() {
        // Extra commas stay in the value field.
        assert_eq!(
            parse_line("Label,icon,cmd --flag=a,b,c"),
            Some(("Label", Some("icon"), Some("cmd --flag=a,b,c")))
        );
    }
}
