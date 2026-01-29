#[macro_export]
macro_rules! get_natural_cmp {
    () => {{
        let mut prefs_num_on = icu_collator::CollatorPreferences::default();
        prefs_num_on.numeric_ordering =
            Some(icu_collator::preferences::CollationNumericOrdering::True);
        icu_collator::Collator::try_new(prefs_num_on, Default::default()).unwrap()
    }};
}
