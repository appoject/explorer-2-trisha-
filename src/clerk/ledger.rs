//! Clerk's running tally of every basic resource he's mined. He never
//! removes anything from here -- he can't combine, so once something is
//! mined it just adds to a permanent per-type count. This is what
//! answers "which resource type do I have the most of, right now".

use std::collections::HashMap;

use common_game::components::resource::BasicResourceType;

#[derive(Debug, Default)]
pub struct Ledger {
    counts: HashMap<BasicResourceType, u32>,
}

impl Ledger {
    /// Records one freshly mined unit of `resource`. Returns
    /// `Some((leader, count))` if the *identity* of the current leader
    /// changed as a result of this addition (including the very first
    /// resource ever mined) -- `None` if the same type is still in the
    /// lead as before, so the caller can log only on real changes rather
    /// than on every single mine.
    pub fn record(&mut self, resource: BasicResourceType) -> Option<(BasicResourceType, u32)> {
        let before = self.leader().map(|(bt, _)| bt);
        *self.counts.entry(resource).or_insert(0) += 1;
        let after = self.leader();

        match after {
            Some((bt, _)) if Some(bt) != before => after,
            _ => None,
        }
    }

    pub fn counts(&self) -> &HashMap<BasicResourceType, u32> {
        &self.counts
    }

    /// The current leader: highest count so far. Ties are broken by
    /// comparing `Debug` output, purely so the result is deterministic
    /// rather than dependent on `HashMap` iteration order -- it carries
    /// no semantic meaning about the resource types themselves.
    pub fn leader(&self) -> Option<(BasicResourceType, u32)> {
        self.counts.iter().map(|(&bt, &c)| (bt, c)).max_by(|a, b| {
            a.1.cmp(&b.1).then_with(|| format!("{:?}", b.0).cmp(&format!("{:?}", a.0)))
        })
    }
}