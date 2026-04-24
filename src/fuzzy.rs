use std::{cell::RefCell, cmp, rc::Rc};

use fuzzy_matcher::{FuzzyMatcher, skim::SkimMatcherV2 as Matcher};
use glib::object::Cast;
use gtk4::{CustomFilter, CustomSorter, Entry, prelude::EditableExt};

use crate::menu_entry::MenuEntry;

pub fn create_fuzzy_filter(entry: Rc<Entry>, matcher: Rc<RefCell<Matcher>>) -> CustomFilter {
    CustomFilter::new(move |item| {
        let menu_entry = item.downcast_ref::<MenuEntry>().unwrap();
        fuzzy_matches(&matcher.borrow(), &menu_entry.label(), entry.text().as_ref())
    })
}

pub fn create_fuzzy_sorter(entry: Rc<Entry>, matcher: Rc<RefCell<Matcher>>) -> CustomSorter {
    CustomSorter::new(move |item_a, item_b| {
        let entry_a = item_a.downcast_ref::<MenuEntry>().unwrap();
        let entry_b = item_b.downcast_ref::<MenuEntry>().unwrap();
        compare_matches(
            &matcher.borrow(),
            &entry_a.label(),
            &entry_b.label(),
            entry.text().as_ref(),
        )
        .into()
    })
}

pub(crate) fn sanitize_text_for_matching(text: &str) -> String {
    text.chars()
        .filter(|c| !c.is_control() || c.is_whitespace())
        .collect::<String>()
        .trim()
        .to_string()
}

pub(crate) fn is_valid_for_matching(text: &str) -> bool {
    !text.is_empty() && text.len() <= 1000 && text.chars().all(|c| c.len_utf8() <= 4)
}

pub(crate) fn fuzzy_matches(matcher: &Matcher, haystack: &str, needle: &str) -> bool {
    let haystack = sanitize_text_for_matching(haystack);
    let needle = sanitize_text_for_matching(needle);

    if needle.is_empty() {
        return true;
    }

    if !is_valid_for_matching(&haystack) || !is_valid_for_matching(&needle) {
        return true;
    }

    matcher.fuzzy_match(&haystack, &needle).is_some()
}

pub(crate) fn compare_matches(
    matcher: &Matcher,
    haystack_a: &str,
    haystack_b: &str,
    needle: &str,
) -> cmp::Ordering {
    let haystack_a = sanitize_text_for_matching(haystack_a);
    let haystack_b = sanitize_text_for_matching(haystack_b);
    let needle = sanitize_text_for_matching(needle);

    if needle.is_empty() {
        return cmp::Ordering::Equal;
    }

    if !is_valid_for_matching(&haystack_a)
        || !is_valid_for_matching(&haystack_b)
        || !is_valid_for_matching(&needle)
    {
        return cmp::Ordering::Equal;
    }

    let score_a = matcher.fuzzy_match(&haystack_a, &needle);
    let score_b = matcher.fuzzy_match(&haystack_b, &needle);

    match (score_a, score_b) {
        (Some(a), Some(b)) => b.cmp(&a),
        (Some(_), None) => cmp::Ordering::Less,
        (None, Some(_)) => cmp::Ordering::Greater,
        (None, None) => cmp::Ordering::Equal,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn matcher() -> Matcher {
        Matcher::default()
    }

    #[test]
    fn sanitize_strips_control_chars_but_keeps_whitespace() {
        assert_eq!(sanitize_text_for_matching("hello\x00world"), "helloworld");
        assert_eq!(sanitize_text_for_matching("foo\tbar"), "foo\tbar");
        assert_eq!(sanitize_text_for_matching("  pad  "), "pad");
    }

    #[test]
    fn sanitize_on_empty_and_control_only() {
        assert_eq!(sanitize_text_for_matching(""), "");
        assert_eq!(sanitize_text_for_matching("\x00\x01\x02"), "");
    }

    #[test]
    fn is_valid_rejects_empty_and_oversized() {
        assert!(!is_valid_for_matching(""));
        assert!(is_valid_for_matching("a"));
        assert!(is_valid_for_matching(&"a".repeat(1000)));
        assert!(!is_valid_for_matching(&"a".repeat(1001)));
    }

    #[test]
    fn is_valid_accepts_utf8_up_to_4_bytes() {
        assert!(is_valid_for_matching("café"));
        assert!(is_valid_for_matching("日本語"));
        assert!(is_valid_for_matching("🦀"));
    }

    #[test]
    fn fuzzy_matches_empty_needle_matches_everything() {
        assert!(fuzzy_matches(&matcher(), "anything", ""));
        assert!(fuzzy_matches(&matcher(), "", ""));
    }

    #[test]
    fn fuzzy_matches_invalid_inputs_pass_through() {
        assert!(fuzzy_matches(&matcher(), "", "query"));
        let big = "a".repeat(2000);
        assert!(fuzzy_matches(&matcher(), &big, "query"));
    }

    #[test]
    fn fuzzy_matches_hit_and_miss() {
        assert!(fuzzy_matches(&matcher(), "Firefox Web Browser", "ffox"));
        assert!(!fuzzy_matches(&matcher(), "Firefox", "zzz"));
    }

    #[test]
    fn compare_matches_empty_needle_is_equal() {
        assert_eq!(
            compare_matches(&matcher(), "apple", "banana", ""),
            cmp::Ordering::Equal
        );
    }

    #[test]
    fn compare_matches_invalid_inputs_are_equal() {
        // The validity guard must short-circuit on ANY invalid input. To kill
        // mutations that flip an `||` to `&&` we need each case to fall into
        // the `Equal` branch only because of that specific arm — meaning the
        // "valid" haystack must actually fuzzy-match the needle, so if the
        // guard were weakened we'd observe a non-Equal ordering instead.
        //
        // haystack_a invalid, needle matches haystack_b:
        assert_eq!(
            compare_matches(&matcher(), "", "banana", "b"),
            cmp::Ordering::Equal
        );
        // haystack_b invalid, needle matches haystack_a:
        assert_eq!(
            compare_matches(&matcher(), "apple", "", "a"),
            cmp::Ordering::Equal
        );
        // needle invalid (oversized), both haystacks would otherwise match:
        let oversized = "a".repeat(2000);
        assert_eq!(
            compare_matches(&matcher(), "apple", "banana", &oversized),
            cmp::Ordering::Equal
        );
    }

    #[test]
    fn compare_matches_better_score_sorts_first() {
        // "firefox" vs "fox firedog": exact prefix "fire" matches better in first.
        let a = compare_matches(&matcher(), "firefox", "fox firedog", "fire");
        assert_eq!(a, cmp::Ordering::Less);
        let b = compare_matches(&matcher(), "fox firedog", "firefox", "fire");
        assert_eq!(b, cmp::Ordering::Greater);
    }

    #[test]
    fn compare_matches_only_one_matches() {
        let only_a = compare_matches(&matcher(), "apple", "banana", "apl");
        assert_eq!(only_a, cmp::Ordering::Less);
        let only_b = compare_matches(&matcher(), "banana", "apple", "apl");
        assert_eq!(only_b, cmp::Ordering::Greater);
    }

    #[test]
    fn compare_matches_neither_matches() {
        assert_eq!(
            compare_matches(&matcher(), "apple", "banana", "xyz"),
            cmp::Ordering::Equal
        );
    }
}
