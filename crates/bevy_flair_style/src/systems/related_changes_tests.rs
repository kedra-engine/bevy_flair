//! Inline values are not selector inputs; selector data still invalidates relations.
use super::*;

struct Tree {
    world: World,
    schedule: Schedule,
    ancestor: Entity,
    subject: Entity,
    sibling: Entity,
    descendant: Entity,
}

impl Tree {
    fn new() -> Self {
        let mut world = World::new();
        let node = || {
            (
                Node::default(),
                StyleData::default(),
                StyleFlags::default(),
                StyleMarkers::default(),
            )
        };
        let ancestor = world.spawn(node()).id();
        let subject = world
            .spawn((
                node(),
                ChildOf(ancestor),
                RawInlineStyle::new(Handle::default()),
            ))
            .id();
        let sibling = world.spawn((node(), ChildOf(ancestor))).id();
        let descendant = world.spawn((node(), ChildOf(subject))).id();
        world
            .get::<StyleFlags>(subject)
            .unwrap()
            .recalculate_on_change_flags
            .insert(RecalculateOnChangeFlags::all());
        let mut schedule = Schedule::default();
        schedule.add_systems(recalculate_style_on_related_entities_changes);
        schedule.run(&mut world);
        for entity in [ancestor, subject, sibling, descendant] {
            world.get_mut::<StyleMarkers>(entity).unwrap().set_to_none();
        }
        schedule.run(&mut world);
        Self {
            world,
            schedule,
            ancestor,
            subject,
            sibling,
            descendant,
        }
    }

    fn change_inline(&mut self) {
        self.world
            .entity_mut(self.subject)
            .insert(RawInlineStyle::new(Handle::default()));
    }

    fn assert_related(&self, expected: bool) {
        for entity in [self.ancestor, self.sibling, self.descendant] {
            assert_eq!(
                self.world
                    .get::<StyleMarkers>(entity)
                    .unwrap()
                    .needs_calculate_style(),
                expected,
                "related entity {entity}"
            );
        }
    }
}

#[test]
fn inline_only_change_recalculates_owner_without_selector_relations() {
    let mut tree = Tree::new();
    tree.change_inline();
    tree.schedule.run(&mut tree.world);
    assert!(
        tree.world
            .get::<StyleMarkers>(tree.subject)
            .unwrap()
            .needs_calculate_style()
    );
    tree.assert_related(false);
}

#[test]
fn simultaneous_selector_data_and_inline_change_retains_relational_invalidation() {
    let mut tree = Tree::new();
    tree.change_inline();
    tree.world
        .get_mut::<StyleData>(tree.subject)
        .unwrap()
        .set_changed();
    tree.schedule.run(&mut tree.world);
    tree.assert_related(true);
}

#[test]
fn pending_selector_recalculation_with_inline_change_retains_relational_invalidation() {
    let mut tree = Tree::new();
    tree.change_inline();
    tree.world
        .get_mut::<StyleMarkers>(tree.subject)
        .unwrap()
        .recalculate_style();
    tree.schedule.run(&mut tree.world);
    tree.assert_related(true);
}

#[test]
fn inline_change_preserves_pending_reset_without_invalidating_selector_relations() {
    let mut tree = Tree::new();
    tree.change_inline();
    tree.world
        .get_mut::<StyleMarkers>(tree.subject)
        .unwrap()
        .reset();
    tree.schedule.run(&mut tree.world);
    assert!(
        tree.world
            .get::<StyleMarkers>(tree.subject)
            .unwrap()
            .needs_reset()
    );
    tree.assert_related(false);
}
