//! Mandatory subject rejection must avoid unrelated relational traversal without
//! changing full matching or the dependency flags of real candidates.
use super::*;
use crate::components::{RecalculateOnChangeFlags, StyleData, StyleFlags};
use bevy_ecs::{prelude::*, system::SystemState};

fn live_match(world: &mut World, subject: Entity, selector: &CssSelector) -> bool {
    let mut state = SystemState::<ElementRefSystemParam>::new(world);
    let params = state.get(world).expect("valid selector queries");
    selector.matches_selector(&ElementRef::new(subject, &params))
}

#[test]
fn absent_subject_keys_do_not_register_unrelated_relational_dependencies() {
    let mut world = World::new();
    let subject = world
        .spawn((StyleData::default(), StyleFlags::default()))
        .id();
    let child = world
        .spawn((
            StyleData {
                classes: vec!["needle".into()],
                ..Default::default()
            },
            StyleFlags::default(),
            ChildOf(subject),
        ))
        .id();
    for text in [
        ".target:has(.needle)",
        "#target:has(.needle)",
        ":has(.needle).target",
        ":has(.needle)#target",
        ".prefix + .target:has(.needle)",
        ".prefix ~ #target:has(.needle)",
    ] {
        let selector = CssSelector::parse_single(text).expect("valid selector");
        assert!(!live_match(&mut world, subject, &selector));
        assert!(
            world
                .get::<StyleFlags>(child)
                .expect("child flags")
                .recalculate_on_change_flags
                .load()
                .is_empty()
        );
        assert!(
            world
                .get::<StyleFlags>(subject)
                .expect("subject flags")
                .css_selector_flags
                .load()
                .is_empty()
        );
    }
    // A negative match is not retained across class/identity changes. A real
    // candidate must still register the engine's normal relational dependencies.
    let mut data = world.get_mut::<StyleData>(subject).expect("subject data");
    data.classes.push("target".into());
    data.id = Some("target".into());
    for text in [".target:has(.needle)", "#target:has(.needle)"] {
        assert!(live_match(
            &mut world,
            subject,
            &CssSelector::parse_single(text).expect("selector")
        ));
    }
    assert!(
        world
            .get::<StyleFlags>(child)
            .expect("child flags")
            .recalculate_on_change_flags
            .load()
            .contains(RecalculateOnChangeFlags::RECALCULATE_ANCESTORS)
    );
}

#[test]
fn subject_rejection_preserves_combinators_alternatives_case_and_pseudo_elements() {
    use crate::css_selector::testing::TestNodeRef;
    let mut tree = ego_tree::Tree::new(StyleData {
        classes: vec!["container".into()],
        id: Some("root".into()),
        ..Default::default()
    });
    let first = tree
        .root_mut()
        .append(StyleData {
            classes: vec!["foo".into(), "世界".into()],
            id: Some("Exact".into()),
            ..Default::default()
        })
        .id();
    tree.get_mut(first).expect("first").append(StyleData {
        classes: vec!["needle".into()],
        ..Default::default()
    });
    tree.root_mut().append(StyleData {
        classes: vec!["bar".into()],
        ..Default::default()
    });
    tree.get_mut(first).expect("first").append(StyleData {
        is_pseudo_element: Some(crate::components::PseudoElement::Before),
        ..Default::default()
    });
    for text in [
        ".foo:has(.needle)",
        ".missing:has(.needle)",
        "#Exact:has(.needle)",
        "#exact:has(.needle)",
        ".世界",
        ".container > .foo",
        ".foo + .bar",
        ".foo ~ .bar",
        ":is(.missing,.foo)",
        ":where(.foo,.bar)",
        ":not(.missing)",
        ".foo:not(.bar)",
        ".container:has(> .foo)",
        ".foo:has(+ .bar)",
        ".foo:has(~ .bar)",
        ":nth-child(2 of .foo,.bar)",
        ".foo::before",
        ".container:has(.missing), .bar",
    ] {
        let mut input = cssparser::ParserInput::new(text);
        let selectors = CssSelector::parse_comma_separated(&mut cssparser::Parser::new(&mut input))
            .expect("valid selector list");
        for selector in selectors {
            for node in tree.nodes() {
                let element: TestNodeRef = node.into();
                let mut caches = SelectorCaches::default();
                let mut context = MatchingContext::new(
                    MatchingMode::Normal,
                    None,
                    &mut caches,
                    QuirksMode::NoQuirks,
                    NeedsSelectorFlags::Yes,
                    MatchingForInvalidation::No,
                );
                let expected = selectors::matching::matches_selector(
                    &selector.selector,
                    0,
                    None,
                    &element,
                    &mut context,
                );
                assert_eq!(
                    selector.matches_selector(&element),
                    expected,
                    "{text}: {:?}",
                    node.id()
                );
            }
        }
    }
}
