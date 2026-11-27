//! Back and forward through the views (27/11/2026): the mouse's side buttons, Alt+← / Alt+→
//! (⌘[ / ⌘] on macOS), as in a browser or a file manager. A view is the tab, the settings page,
//! what the vault or the contacts show, and the item or contact open in the panel.

use crate::app::{CleisApp, Filter, Scope, Session, Tab};
use cleis_core::{ContactId, EntryId};
use gpui_kit::{Context, NavigationDirection};
use std::time::{Duration, Instant};

/// How many views are kept behind.
const DEPTH: usize = 50;
/// Selections closer than this (the arrow keys going down the list) are one step.
const BURST: Duration = Duration::from_millis(800);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct View {
    tab: Tab,
    settings_page: &'static str,
    filter: Filter,
    scope: Scope,
    selected: Option<EntryId>,
    contact: Option<ContactId>,
}

impl View {
    fn of(s: &Session) -> Self {
        Self {
            tab: s.tab,
            settings_page: s.settings_page,
            filter: s.filter,
            scope: s.scope.clone(),
            selected: s.selected,
            contact: s.selected_contact,
        }
    }

    /// Only the item or contact open differs.
    fn same_place(&self, o: &Self) -> bool {
        self.tab == o.tab
            && self.settings_page == o.settings_page
            && self.filter == o.filter
            && self.scope == o.scope
    }
}

#[derive(Default)]
pub struct History {
    back: Vec<View>,
    forward: Vec<View>,
    /// The view seen at the last render.
    seen: Option<View>,
    changed: Option<Instant>,
}

impl History {
    pub fn can_go(&self, dir: NavigationDirection) -> bool {
        match dir {
            NavigationDirection::Back => !self.back.is_empty(),
            NavigationDirection::Forward => !self.forward.is_empty(),
        }
    }
}

/// Called at each render: a view that changed since the last one goes behind.
pub fn track(s: &mut Session) {
    let now = View::of(s);
    let h = &mut s.history;
    let Some(seen) = h.seen.replace(now.clone()) else {
        return;
    };
    if seen == now {
        return;
    }
    let burst = h.changed.is_some_and(|t| t.elapsed() < BURST);
    h.changed = Some(Instant::now());
    h.forward.clear();
    // The arrow keys through a list: one step back to where they started.
    if burst && seen.same_place(&now) && h.back.last().is_some_and(|b| b.same_place(&seen)) {
        return;
    }
    h.back.push(seen);
    if h.back.len() > DEPTH {
        h.back.remove(0);
    }
}

impl CleisApp {
    /// One view back or forward. Returns whether there was one.
    pub fn navigate(&mut self, dir: NavigationDirection, cx: &mut Context<Self>) -> bool {
        let Some(s) = self.session() else {
            return false;
        };
        let current = View::of(s);
        let (from, to) = match dir {
            NavigationDirection::Back => (&mut s.history.back, &mut s.history.forward),
            NavigationDirection::Forward => (&mut s.history.forward, &mut s.history.back),
        };
        // A view whose item or contact is gone is passed over to the one before.
        let Some(mut v) = from.pop() else {
            return false;
        };
        to.push(current);
        if v.selected
            .is_some_and(|id| !s.entries.iter().any(|e| e.id == id))
        {
            v.selected = None;
        }
        if v.contact
            .is_some_and(|id| !s.contacts.iter().any(|c| c.id == id))
        {
            v.contact = None;
        }
        // An organization left meanwhile: its scope is no longer there.
        if let Scope::Org(k) = &v.scope
            && !s.all_orgs().any(|o| &o.m.key() == k)
        {
            v.scope = Scope::All;
        }
        s.tab = v.tab;
        s.settings_page = v.settings_page;
        s.filter = v.filter;
        s.scope = v.scope.clone();
        if s.selected != v.selected {
            s.reveal = false;
        }
        s.selected = v.selected;
        s.selected_contact = v.contact;
        s.ctx = None;
        s.history.seen = Some(v);
        s.history.changed = None;
        s.refilter();
        cx.notify();
        true
    }
}
