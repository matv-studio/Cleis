//! Autofill in the app (D-31, screens 16, 17): global shortcuts, Quick Access window, fills
//! running in the background, and the steps kept for the next press (password page, 2FA).

use crate::app::{CleisApp, Phase};
use crate::screens::quick_access::QuickAccess;
use cleis_autofill::hotkeys::{Hotkeys, Shortcut, Status};
use cleis_autofill::{FillError, Foreground, Outcome, Values, WindowRef};
use cleis_core::EntryId;
use cleis_core::autofill::{self as logic, Action, Focus, Plan, PlanOptions};
use cleis_core::settings::FillMethod;
use gpui_kit::prelude::*;
use gpui_kit::{
    AnyWindowHandle, Bounds, Context, Task, WindowBounds, WindowKind, WindowOptions, px, size,
};
use std::time::{Duration, Instant};
use zeroize::Zeroizing;

/// Steps waiting for the next "Fill best match" on the same window.
pub struct Pending {
    pub window: WindowRef,
    pub entry: EntryId,
    /// Steps for the next press (the password page).
    pub actions: Vec<Action>,
    /// Steps for the press after that (the 2FA page).
    pub later: Vec<Action>,
    pub until: Instant,
}

/// How long a pending step stays valid: a password page or a 2FA prompt comes within seconds.
const PENDING_FOR: Duration = Duration::from_secs(120);

#[derive(Default)]
pub struct AutofillState {
    hotkeys: Option<Hotkeys>,
    pub status: Option<[(Shortcut, Status); 2]>,
    pub pending: Option<Pending>,
    pub quick_access: Option<AnyWindowHandle>,
    /// What was asked to the shortcuts last time, to re-register only on change.
    registered: Option<(Option<String>, Option<String>)>,
    _listener: Option<Task<()>>,
    /// Tests and screenshots: fills are recorded here instead of reaching the system.
    pub dry_run: Option<Vec<(EntryId, Option<FillMethod>)>>,
}

/// Empty plan: [`CleisApp::start_fill`] then uses the standard sign-in sequence of the item.
pub fn standard(_: Focus) -> Plan {
    Plan {
        now: vec![],
        later: vec![],
    }
}

/// What a fill needs from an item, copied out of the session for the background thread.
struct Job {
    entry: EntryId,
    values: Values,
    has_username: bool,
    has_totp: bool,
}

impl CleisApp {
    /// Registers or releases the shortcuts after unlocking or a change in Settings › Autofill.
    /// Shortcuts stay registered while locked: pressing one then brings the unlock screen.
    pub fn sync_autofill(&mut self, cx: &mut Context<Self>) {
        let Phase::Main(s) = &self.phase else { return };
        let a = &s.prefs.autofill;
        let wanted = if a.enabled {
            (
                a.quick_access.enabled.then(|| a.quick_access.keys.clone()),
                a.fill_best_match
                    .enabled
                    .then(|| a.fill_best_match.keys.clone()),
            )
        } else {
            (None, None)
        };
        if self.autofill.registered.as_ref() == Some(&wanted) {
            return;
        }
        if self.autofill.hotkeys.is_none() {
            match Hotkeys::new() {
                Ok(h) => self.autofill.hotkeys = Some(h),
                Err(e) => {
                    let msg = format!("Shortcuts unavailable: {e}");
                    self.autofill.status = Some([
                        (Shortcut::QuickAccess, Status::Unavailable(msg.clone())),
                        (Shortcut::FillBestMatch, Status::Unavailable(msg)),
                    ]);
                    return;
                }
            }
        }
        let Some(h) = self.autofill.hotkeys.as_mut() else {
            return;
        };
        self.autofill.status = Some(h.set(wanted.0.as_deref(), wanted.1.as_deref()));
        self.autofill.registered = Some(wanted);
        let (tx, rx) = async_channel::unbounded::<Shortcut>();
        h.listen(move |s| {
            let _ = tx.send_blocking(s);
        });
        self.autofill._listener = Some(cx.spawn(async move |this, cx| {
            while let Ok(s) = rx.recv().await {
                if this.update(cx, |app, cx| app.on_shortcut(s, cx)).is_err() {
                    break;
                }
            }
        }));
    }

    fn on_shortcut(&mut self, which: Shortcut, cx: &mut Context<Self>) {
        let Some(fg) = cleis_autofill::foreground() else {
            return;
        };
        // Pressed inside Cleis itself: nothing to fill.
        if fg.context.program.starts_with("cleis") {
            return;
        }
        let Phase::Main(s) = &self.phase else {
            // Locked: bring the unlock screen (also when hidden to the tray).
            self.show_main(cx);
            return;
        };
        let read_url = fg.context.is_browser() && s.prefs.autofill.match_browser_urls;
        cx.spawn(async move |this, cx| {
            let mut fg = fg;
            if read_url {
                let w = fg.window;
                fg.context.url = cx
                    .background_spawn(async move { cleis_autofill::browser_url(w) })
                    .await;
            }
            this.update(cx, |app, cx| match which {
                Shortcut::QuickAccess => app.open_quick_access(fg, cx),
                Shortcut::FillBestMatch => app.fill_best(fg, cx),
            })
            .ok();
        })
        .detach();
    }

    fn plan_options(&mut self) -> PlanOptions {
        let a = self
            .session()
            .map(|s| s.prefs.autofill.clone())
            .unwrap_or_default();
        PlanOptions {
            press_enter: a.press_enter,
            totp_after: a.fill_totp_after,
        }
    }

    fn fill_best(&mut self, fg: Foreground, cx: &mut Context<Self>) {
        // A password page or 2FA prompt following a previous fill on this window.
        if let Some(p) = self.autofill.pending.take()
            && p.window == fg.window
            && p.until > Instant::now()
        {
            let (now, later) = (p.actions, p.later);
            let next = move |_| Plan { now, later };
            self.start_fill(p.entry, fg, false, next, cx);
            return;
        }
        let Some(s) = self.session() else { return };
        let ask = s.prefs.autofill.ask_unlinked;
        let ranked = logic::rank(&s.entries, &fg.context);
        match logic::best(&ranked, ask) {
            Some(id) => self.start_fill(id, fg, false, standard, cx),
            None => self.open_quick_access(fg, cx),
        }
    }

    /// Fills `entry` into the window of `fg`. `custom` gives the plan from the focused field;
    /// an empty plan means "the standard sign-in sequence for this item".
    pub fn start_fill(
        &mut self,
        entry: EntryId,
        fg: Foreground,
        from_quick_access: bool,
        custom: impl FnOnce(Focus) -> Plan + Send + 'static,
        cx: &mut Context<Self>,
    ) {
        self.start_fill_with(entry, fg, from_quick_access, None, custom, cx);
    }

    /// [`Self::start_fill`] with the filling method forced (Quick Access: Shift+Enter types).
    pub fn start_fill_with(
        &mut self,
        entry: EntryId,
        fg: Foreground,
        from_quick_access: bool,
        method: Option<FillMethod>,
        custom: impl FnOnce(Focus) -> Plan + Send + 'static,
        cx: &mut Context<Self>,
    ) {
        if let Some(e) = self
            .session()
            .and_then(|s| s.entries.iter().find(|e| e.id == entry).cloned())
        {
            self.org_report(cleis_core::org::Action::Filled, &e, "", cx);
        }
        if let Some(log) = self.autofill.dry_run.as_mut() {
            log.push((entry, method));
            if from_quick_access {
                self.learn_link(entry, &fg, cx);
            }
            return;
        }
        let o = self.plan_options();
        let Some(s) = self.session() else { return };
        let Some(e) = s.entries.iter().find(|e| e.id == entry) else {
            return;
        };
        let now = cleis_core::now();
        let job = Job {
            entry,
            has_username: !e.username.is_empty(),
            has_totp: e.totp.is_some(),
            values: Values {
                username: Zeroizing::new(e.username.clone()),
                password: Zeroizing::new(e.password.clone()),
                totp: e.totp.as_ref().map(|t| Zeroizing::new(t.code_at(now))),
            },
        };
        let method = method.unwrap_or(s.prefs.autofill.method);
        let speed = s.prefs.autofill.typing_speed;
        let window = fg.window;
        let result = cx.background_spawn(async move {
            let (has_user, has_totp) = (job.has_username, job.has_totp);
            let r = cleis_autofill::fill(window, &job.values, method, speed, move |focus| {
                let p = custom(focus);
                if p.now.is_empty() && p.later.is_empty() {
                    logic::plan(has_user, has_totp, focus, o)
                } else {
                    p
                }
            });
            (job.entry, r)
        });
        cx.spawn(async move |this, cx| {
            let (entry, r) = result.await;
            this.update(cx, |app, cx| {
                app.fill_done(entry, fg, from_quick_access, r, cx)
            })
            .ok();
        })
        .detach();
    }

    fn fill_done(
        &mut self,
        entry: EntryId,
        fg: Foreground,
        from_quick_access: bool,
        r: Result<(Outcome, Vec<Action>), FillError>,
        cx: &mut Context<Self>,
    ) {
        match r {
            Ok((outcome, later)) => {
                // Stopped before the password (two-page sign-in): the password waits for the
                // next press, the 2FA code for the one after. Done: the 2FA code waits.
                let (actions, later) = match outcome {
                    Outcome::Done => (later, vec![]),
                    Outcome::Waiting(rest) => (rest, later),
                };
                self.autofill.pending = (!actions.is_empty()).then(|| Pending {
                    window: fg.window,
                    entry,
                    actions,
                    later,
                    until: Instant::now() + PENDING_FOR,
                });
                if from_quick_access {
                    self.learn_link(entry, &fg, cx);
                }
            }
            Err(e) => {
                self.autofill.pending = None;
                self.toast_for(format!("Not filled: {e}"), 10, cx);
            }
        }
    }

    /// First fill of a desktop app from Quick Access links it to the item (screen 16).
    ///
    /// Only the first item chosen for an app is linked: picking another one later (a second
    /// account, a search) does not add links behind the user's back.
    fn learn_link(&mut self, entry: EntryId, fg: &Foreground, cx: &mut Context<Self>) {
        let Some(s) = self.session() else { return };
        let program = &fg.context.program;
        if s.entries.iter().any(|e| {
            e.linked_apps
                .iter()
                .any(|l| l.program.eq_ignore_ascii_case(program))
        }) {
            return;
        }
        let Some(e) = s.entries.iter_mut().find(|e| e.id == entry) else {
            return;
        };
        if logic::learn_link(e, &fg.context) {
            e.touch(None);
            if let Err(err) = s.vault.put_entry(e) {
                s.toast = Some(format!("Link not saved: {err}").into());
            }
            cx.notify();
        }
    }

    pub fn unlink_app(&mut self, entry: EntryId, program: String, cx: &mut Context<Self>) {
        let Some(s) = self.session() else { return };
        let Some(e) = s.entries.iter_mut().find(|e| e.id == entry) else {
            return;
        };
        e.linked_apps
            .retain(|l| !l.program.eq_ignore_ascii_case(&program));
        e.touch(None);
        if let Err(err) = s.vault.put_entry(e) {
            s.toast = Some(format!("Not saved: {err}").into());
        }
        cx.notify();
    }

    pub fn open_quick_access(&mut self, fg: Foreground, cx: &mut Context<Self>) {
        if let Some(w) = self.autofill.quick_access.take() {
            w.update(cx, |_, window, _| window.remove_window()).ok();
        }
        let me = cx.entity().downgrade();
        // On the screen of the window being filled, not always the main one.
        let display = fg.monitor.and_then(|m| {
            cx.displays()
                .into_iter()
                .map(|d| d.id())
                .find(|id| u64::from(*id) == m)
        });
        let bounds = Bounds::centered(display, size(px(600.), px(420.)), cx);
        let opts = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            display_id: display,
            titlebar: None,
            kind: WindowKind::PopUp,
            focus: true,
            show: true,
            is_movable: false,
            is_resizable: false,
            is_minimizable: false,
            ..Default::default()
        };
        // Opened once this update is over: Quick Access reads the app while it draws.
        cx.defer(move |cx| {
            let me2 = me.clone();
            let r = gpui_kit::open_window(opts, cx, move |window, cx| {
                cx.new(|cx| QuickAccess::new(me2, fg, window, cx))
            });
            me.update(cx, |a, cx| match r {
                Ok((handle, _)) => {
                    a.autofill.quick_access = Some(handle);
                    cx.activate(true);
                }
                Err(e) => a.toast(format!("Quick Access unavailable: {e}"), cx),
            })
            .ok();
        });
    }

    /// Called by Quick Access when it closes (fill chosen, Escape, or lost focus).
    pub fn quick_access_closed(&mut self) {
        self.autofill.quick_access = None;
    }
}
