//! The server driven by the real client over TLS on 127.0.0.1: joining with an invitation code,
//! signing in, sharing a space, pushing and pulling sealed rows, rights, verification codes,
//! revocation, re-encryption, the activity log and backups.

use cleis_core::org::*;
use cleis_core::{MemberId, SpaceId};
use cleis_crypto::{KeyPair, SigningKeyPair, SymmetricKey, seal, unwrap_key, wrap_key};
use cleis_net::{Client, NetError, Trust};
use cleis_server::App;
use std::path::PathBuf;
use std::sync::Arc;

struct Server {
    addr: String,
    dir: PathBuf,
    stop: Option<tokio::sync::oneshot::Sender<()>>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl Server {
    fn start(dir: PathBuf) -> Self {
        let app = App::open(&dir).unwrap();
        Self::start_app(app, dir)
    }

    fn start_app(app: Arc<App>, dir: PathBuf) -> Self {
        let (tx, rx) = tokio::sync::oneshot::channel::<()>();
        let (atx, arx) = std::sync::mpsc::channel();
        let thread = std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async move {
                let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
                atx.send(l.local_addr().unwrap().to_string()).unwrap();
                cleis_server::serve(app, l, async {
                    let _ = rx.await;
                })
                .await
                .unwrap();
            });
        });
        Self {
            addr: arx.recv().unwrap(),
            dir,
            stop: Some(tx),
            thread: Some(thread),
        }
    }

    fn stop(mut self) -> PathBuf {
        let _ = self.stop.take().unwrap().send(());
        self.thread.take().unwrap().join().unwrap();
        self.dir.clone()
    }
}

struct Person {
    account: KeyPair,
    signing: SigningKeyPair,
    /// The organization's PIN given at sign-in (D-79).
    pin: Option<String>,
    member: MemberId,
    client: Client,
}

fn join(addr: &str, code: &str, name: &str, email: &str) -> Result<Person, NetError> {
    let inv = Invitation::parse(code).unwrap();
    let account = KeyPair::generate().unwrap();
    let signing = SigningKeyPair::generate().unwrap();
    let mut req = JoinRequest {
        token: inv.token.to_vec(),
        name: name.into(),
        email: email.into(),
        device: format!("PC-{name}"),
        account_public: account.public().0,
        verify_key: signing.verifying_key().0,
        signature: [0; 64],
        proof: vec![],
    };
    req.signature = signing.sign(JOIN_DOMAIN, &req.signed_bytes());
    let first = Client::new(addr, Trust::Prefix(inv.pin));
    let reply: JoinReply = first.call("/v1/join", &req)?;
    Ok(Person {
        account,
        signing,
        pin: None,
        member: reply.member,
        client: Client::new(addr, Trust::Pinned(vec![reply.org.cert])),
    })
}

impl Person {
    fn sign_in(&mut self, code: Option<String>) -> Result<SessionReply, NetError> {
        self.sign_in_with(code, None)
    }

    fn sign_in_with(
        &mut self,
        code: Option<String>,
        id_token: Option<String>,
    ) -> Result<SessionReply, NetError> {
        let c: Challenge = self.client.call("/v1/challenge", &())?;
        let r = self.client.send("POST", "/v1/hello", None);
        let cert = match r {
            Ok(r) => r.cert,
            Err(_) => [0; 32],
        };
        let sig = self
            .signing
            .sign(SIGN_IN_DOMAIN, &sign_in_message(&c.nonce, &cert));
        let reply: SessionReply = self.client.call(
            "/v1/session",
            &SessionRequest {
                member: self.member,
                nonce: c.nonce,
                signature: sig,
                device: "PC".into(),
                code,
                remembered: None,
                id_token,
                pin: self.pin.clone(),
            },
        )?;
        self.client.token = Some(reply.token.clone());
        Ok(reply)
    }

    fn state(&self) -> State {
        self.client.get("/v1/state").unwrap()
    }
}

fn row(space: SpaceId, key: &SymmetricKey, id: &str, text: &str, changed: i64, gen_: u32) -> Row {
    Row {
        kind: "entry".into(),
        id: id.into(),
        space,
        sealed: seal(
            key,
            format!("cleis/entry/{id}/{space}").as_bytes(),
            text.as_bytes(),
        )
        .unwrap(),
        rev: 1,
        updated: 0,
        deleted: None,
        changed,
        key_gen: gen_,
    }
}

fn tempdir() -> PathBuf {
    let d = std::env::temp_dir().join(format!("cleis-server-test-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn organization_flow() {
    let dir = tempdir().join("data");
    let code = App::init(&dir, "Example Studio", &[]).unwrap();
    let srv = Server::start(dir.clone());

    // A code with a wrong pin is refused at the TLS level.
    let mut bad = Invitation::parse(&code).unwrap();
    bad.pin[0] ^= 1;
    let e = join(&srv.addr, &bad.code(), "Lea", "lea@example-studio.com")
        .err()
        .unwrap();
    assert!(matches!(e, NetError::Untrusted), "{e:?}");

    // What the code is for.
    let inv = Invitation::parse(&code).unwrap();
    let info: InviteInfo = Client::new(&srv.addr, Trust::Prefix(inv.pin))
        .call(
            "/v1/invite-info",
            &InviteLookup {
                token: inv.token.to_vec(),
            },
        )
        .unwrap();
    assert_eq!(
        (info.org.as_str(), info.name.as_str()),
        ("Example Studio", "")
    );

    // The first admin joins and sets the organization key.
    let mut lea = join(&srv.addr, &code, "Lea Martin", "lea@example-studio.com").unwrap();
    // The code is used up.
    assert!(join(&srv.addr, &code, "X", "x@example-studio.com").is_err());
    lea.sign_in(None).unwrap();
    let st = lea.state();
    assert_eq!(st.org.name, "Example Studio");
    assert_eq!(st.me.role, SUPER_ADMIN);
    assert!(st.org.org_public.is_none());
    let org = KeyPair::generate().unwrap();
    let org_secret = SymmetricKey::from_bytes(*org.secret_bytes());
    let _: PushReply = lea
        .client
        .call(
            "/v1/admin-keys",
            &AdminKeys {
                org_public: Some(org.public().0.to_vec()),
                wraps: vec![Wrap {
                    space: SpaceId(uuid::Uuid::nil()),
                    member: lea.member,
                    wrapped: wrap_key(&lea.account.public(), &org_secret, b"cleis/org-key")
                        .unwrap(),
                }],
            },
        )
        .unwrap();
    let st = lea.state();
    assert!(st.org_key.is_some() && st.org.org_public.is_some());

    // A Standard space: key wrapped to Lea and to the organization key.
    let general = SpaceId::new();
    let k1 = SymmetricKey::generate().unwrap();
    let _: PushReply = lea
        .client
        .call(
            "/v1/spaces",
            &NewSpace {
                id: general,
                sealed: seal(&k1, b"space", b"General").unwrap(),
                mode: SpaceMode::Standard,
                reserved: false,
                wrapped: wrap_key(&lea.account.public(), &k1, general.0.as_bytes()).unwrap(),
                org_wrapped: Some(wrap_key(&org.public(), &k1, general.0.as_bytes()).unwrap()),
                recovery_wrapped: None,
            },
        )
        .unwrap();
    let r: PushReply = lea
        .client
        .call(
            "/v1/push",
            &Changes {
                rows: vec![row(general, &k1, "a", "Figma", 1000, 1)],
                purged: vec![],
                seq: 0,
            },
        )
        .unwrap();
    assert!(r.refused.is_empty());

    // Lea invites Hugo as Read only in General.
    let inv: InviteReply = lea
        .client
        .call(
            "/v1/invites",
            &NewInvite {
                name: "Hugo Bernard".into(),
                email: "hugo@example-studio.com".into(),
                role: "read_only".into(),
                spaces: vec![general],
                address: String::new(),
                secret: None,
            },
        )
        .unwrap();
    assert_eq!(lea.state().invites.len(), 1);
    let mut hugo = join(&srv.addr, &inv.code, "", "").unwrap();
    hugo.sign_in(None).unwrap();
    let st = hugo.state();
    assert_eq!(st.me.name, "Hugo Bernard");
    assert!(
        st.spaces[0].wrapped.is_none(),
        "no key until someone wraps it"
    );
    // Hugo cannot manage.
    let e = hugo
        .client
        .call::<_, InviteReply>(
            "/v1/invites",
            &NewInvite {
                name: String::new(),
                email: "x@example-studio.com".into(),
                role: "member".into(),
                spaces: vec![],
                address: String::new(),
                secret: None,
            },
        )
        .unwrap_err();
    assert_eq!(e.code(), Some("forbidden"));

    // Lea's app wraps the key for the pending member.
    let st = lea.state();
    assert_eq!(st.pending.len(), 1);
    let p = &st.pending[0];
    let _: PushReply = lea
        .client
        .call(
            "/v1/grants",
            &Grants {
                wraps: vec![Wrap {
                    space: p.space,
                    member: p.member,
                    wrapped: wrap_key(
                        &cleis_crypto::PublicKey(p.account_public),
                        &k1,
                        general.0.as_bytes(),
                    )
                    .unwrap(),
                }],
            },
        )
        .unwrap();
    let st = hugo.state();
    let hk = unwrap_key(
        &hugo.account,
        st.spaces[0].wrapped.as_ref().unwrap(),
        general.0.as_bytes(),
    )
    .unwrap();
    assert_eq!(hk, k1);
    let ch: Changes = hugo
        .client
        .call(
            "/v1/pull",
            &Pull {
                since: 0,
                full: vec![],
            },
        )
        .unwrap();
    assert_eq!(ch.rows.len(), 1);
    // Read only: writes are refused.
    let r: PushReply = hugo
        .client
        .call(
            "/v1/push",
            &Changes {
                rows: vec![row(general, &k1, "b", "Slack", 2000, 1)],
                purged: vec![],
                seq: 0,
            },
        )
        .unwrap();
    assert_eq!(r.refused.len(), 1);

    // Promoted to Member: Hugo writes; an older copy from Lea is refused, a newer one kept.
    let _: MemberView = lea
        .client
        .call(
            "/v1/members",
            &MemberChange {
                member: hugo.member,
                role: Some("member".into()),
                ..Default::default()
            },
        )
        .unwrap();
    hugo.sign_in(None).unwrap();
    let r: PushReply = hugo
        .client
        .call(
            "/v1/push",
            &Changes {
                rows: vec![row(general, &k1, "a", "Figma 2", 3000, 1)],
                purged: vec![],
                seq: 0,
            },
        )
        .unwrap();
    assert!(r.refused.is_empty());
    let r: PushReply = lea
        .client
        .call(
            "/v1/push",
            &Changes {
                rows: vec![row(general, &k1, "a", "Figma old", 2500, 1)],
                purged: vec![],
                seq: 0,
            },
        )
        .unwrap();
    assert_eq!(r.refused.len(), 1);
    let v: VersionList = lea
        .client
        .call(
            "/v1/versions",
            &Versions {
                kind: "entry".into(),
                id: "a".into(),
            },
        )
        .unwrap();
    assert_eq!(v.rows.len(), 1);
    let ch: Changes = lea
        .client
        .call(
            "/v1/pull",
            &Pull {
                since: 1,
                full: vec![],
            },
        )
        .unwrap();
    assert_eq!(ch.rows.len(), 1);
    assert_eq!(ch.rows[0].changed, 3000);

    // Policy: verification code required. Hugo gets a set-up-only session until he sets it.
    let mut policy = lea.state().policy;
    policy.code_required = true;
    policy.detail = LogDetail::Everything;
    let _: Policy = lea.client.call("/v1/policy", &policy).unwrap();
    let s = hugo.sign_in(None).unwrap();
    assert!(s.setup_code);
    let e = hugo
        .client
        .call::<_, Changes>(
            "/v1/pull",
            &Pull {
                since: 0,
                full: vec![],
            },
        )
        .unwrap_err();
    assert_eq!(e.code(), Some("setup_code"));
    let setup: CodeSetup = hugo.client.call("/v1/me/code", &()).unwrap();
    let totp = cleis_core::totp::Totp::from_base32(&setup.secret).unwrap();
    let now = cleis_core::now();
    let _: SessionReply = hugo
        .client
        .call(
            "/v1/me/code/confirm",
            &CodeCheck {
                code: totp.code_at(now),
            },
        )
        .unwrap();
    let _: Changes = hugo
        .client
        .call(
            "/v1/pull",
            &Pull {
                since: 0,
                full: vec![],
            },
        )
        .unwrap();
    // Next sign-in asks for the code; the code just used is refused (replay).
    assert_eq!(
        hugo.sign_in(None).unwrap_err().code(),
        Some("code_required")
    );
    assert_eq!(
        hugo.sign_in(Some(totp.code_at(now))).unwrap_err().code(),
        Some("wrong_code")
    );
    let ok = hugo.sign_in(Some(totp.code_at(now + 30)));
    assert!(ok.is_ok(), "{ok:?}");
    policy.code_required = false;
    let _: Policy = lea.client.call("/v1/policy", &policy).unwrap();

    // Reported events and exports.
    let _: PushReply = hugo
        .client
        .call(
            "/v1/log",
            &Report {
                action: Action::Copied,
                space: Some(general),
                item: Some("entry/a".into()),
                detail: Some(
                    cleis_crypto::seal_to(&org.public(), DETAIL_CONTEXT, b"{\"item\":\"Figma\"}")
                        .unwrap(),
                ),
            },
        )
        .unwrap();
    let e = hugo
        .client
        .call::<_, PushReply>(
            "/v1/log",
            &Report {
                action: Action::Exported,
                space: Some(general),
                item: None,
                detail: None,
            },
        )
        .unwrap_err();
    assert_eq!(e.code(), Some("forbidden"));
    let page: LogPage = lea
        .client
        .call(
            "/v1/log/query",
            &LogQuery {
                action: Some(Action::Copied),
                limit: 50,
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(page.events.len(), 1);
    let detail = cleis_crypto::open_from(
        &org,
        DETAIL_CONTEXT,
        page.events[0].detail.as_ref().unwrap(),
    )
    .unwrap();
    assert_eq!(&detail[..], b"{\"item\":\"Figma\"}");
    let integrity: Integrity = lea.client.call("/v1/log/verify", &()).unwrap();
    assert!(integrity.intact, "{integrity:?}");
    assert!(integrity.checked >= 8);

    // Super Admin (D-60): Lea makes Hugo Admin; an Admin cannot change admins or roles; the
    // Super Admin role is handed to Hugo and back; the last Super Admin cannot step down.
    let change = |m: MemberId, role: &str| MemberChange {
        member: m,
        role: Some(role.into()),
        ..Default::default()
    };
    let _: MemberView = lea
        .client
        .call("/v1/members", &change(hugo.member, ADMIN))
        .unwrap();
    let e = hugo
        .client
        .call::<_, MemberView>("/v1/members", &change(lea.member, "member"))
        .unwrap_err();
    assert_eq!(e.code(), Some("forbidden"));
    let e = hugo
        .client
        .call::<_, Vec<Role>>("/v1/roles", &Role::defaults())
        .unwrap_err();
    assert_eq!(e.code(), Some("forbidden"));

    // Reserved space (D-61): Lea's "Accounting" is hidden from Hugo, an Admin, who can neither
    // join it, rename it nor delete it. A Standard space cannot be reserved as it is.
    let acc = SpaceId::new();
    let k_acc = SymmetricKey::generate().unwrap();
    let e = hugo
        .client
        .call::<_, PushReply>(
            "/v1/spaces",
            &NewSpace {
                id: SpaceId::new(),
                sealed: vec![1],
                mode: SpaceMode::EndToEnd,
                reserved: true,
                wrapped: vec![1],
                org_wrapped: None,
                recovery_wrapped: None,
            },
        )
        .unwrap_err();
    assert_eq!(e.code(), Some("forbidden"), "only a Super Admin reserves");
    let _: PushReply = lea
        .client
        .call(
            "/v1/spaces",
            &NewSpace {
                id: acc,
                sealed: seal(&k_acc, b"space", b"Accounting").unwrap(),
                mode: SpaceMode::EndToEnd,
                reserved: true,
                wrapped: wrap_key(&lea.account.public(), &k_acc, acc.0.as_bytes()).unwrap(),
                org_wrapped: None,
                recovery_wrapped: None,
            },
        )
        .unwrap();
    assert!(lea.state().spaces.iter().any(|s| s.id == acc && s.reserved));
    assert!(
        hugo.state().spaces.iter().all(|s| s.id != acc),
        "hidden from an Admin"
    );
    let e = hugo
        .client
        .call::<_, MemberView>(
            "/v1/members",
            &MemberChange {
                member: hugo.member,
                spaces: Some(vec![general, acc]),
                ..Default::default()
            },
        )
        .unwrap_err();
    assert_eq!(e.code(), Some("forbidden"));
    for (path, body) in [
        ("/v1/spaces/delete", serde_json::json!({ "id": acc.0 })),
        (
            "/v1/spaces/reserve",
            serde_json::json!({ "id": acc.0, "reserved": false }),
        ),
    ] {
        let e = hugo.client.call::<_, PushReply>(path, &body).unwrap_err();
        assert_eq!(e.code(), Some("forbidden"), "{path}");
    }
    let e = lea
        .client
        .call::<_, PushReply>(
            "/v1/spaces/reserve",
            &Reserve {
                id: general,
                reserved: true,
            },
        )
        .unwrap_err();
    assert_eq!(e.code(), Some("bad_request"));
    // Lea gives Hugo the space herself: allowed, and marked as hers.
    let _: MemberView = lea
        .client
        .call(
            "/v1/members",
            &MemberChange {
                member: hugo.member,
                spaces: Some(vec![general, acc]),
                ..Default::default()
            },
        )
        .unwrap();
    let p = lea
        .state()
        .pending
        .into_iter()
        .find(|p| p.space == acc)
        .unwrap();
    assert_eq!(p.granted_by, Some(lea.member));
    // Given by a Super Admin, the space shows for Hugo; leaving it still needs one.
    assert!(hugo.state().spaces.iter().any(|s| s.id == acc));
    let e = hugo
        .client
        .call::<_, MemberView>(
            "/v1/members",
            &MemberChange {
                member: hugo.member,
                spaces: Some(vec![general]),
                ..Default::default()
            },
        )
        .unwrap_err();
    assert_eq!(e.code(), Some("forbidden"));
    let e = lea
        .client
        .call::<_, MemberView>("/v1/members", &change(lea.member, ADMIN))
        .unwrap_err();
    assert_eq!(e.code(), Some("bad_request"), "last Super Admin");
    let _: MemberView = lea
        .client
        .call("/v1/members/transfer", &IdOnly { id: hugo.member.0 })
        .unwrap();
    let st = lea.state();
    assert_eq!(st.me.role, ADMIN);
    assert_eq!(
        st.members
            .iter()
            .find(|m| m.id == hugo.member)
            .unwrap()
            .role,
        SUPER_ADMIN
    );
    let _: MemberView = hugo
        .client
        .call("/v1/members/transfer", &IdOnly { id: lea.member.0 })
        .unwrap();
    assert_eq!(lea.state().me.role, SUPER_ADMIN);

    // Profile, devices, renaming and deleting a space (D-60).
    let me: MemberView = lea
        .client
        .call(
            "/v1/me",
            &ProfileChange {
                name: "Léa Martin".into(),
                email: "lea@example-studio.com".into(),
                photo: Some("data:image/jpeg;base64,/9j/AA==".into()),
            },
        )
        .unwrap();
    assert_eq!(me.name, "Léa Martin");
    assert_eq!(me.photo.as_deref(), Some("data:image/jpeg;base64,/9j/AA=="));
    // D-101: an older app sends no photo: kept; too large or not a picture: refused; "": gone.
    let me: MemberView = lea
        .client
        .call(
            "/v1/me",
            &ProfileChange {
                name: "Léa Martin".into(),
                email: "lea@example-studio.com".into(),
                photo: None,
            },
        )
        .unwrap();
    assert!(me.photo.is_some());
    // The e-mail is optional: none sent, the member keeps the one the organization has.
    let me: MemberView = lea
        .client
        .call(
            "/v1/me",
            &ProfileChange {
                name: "Léa M.".into(),
                email: String::new(),
                photo: None,
            },
        )
        .unwrap();
    assert_eq!(me.name, "Léa M.");
    assert_eq!(me.email, "lea@example-studio.com");
    for bad in [
        "https://example-studio.com/a.png".to_owned(),
        format!("data:image/png;base64,{}", "A".repeat(PHOTO_MAX)),
    ] {
        assert!(
            lea.client
                .call::<_, MemberView>(
                    "/v1/me",
                    &ProfileChange {
                        name: "Léa Martin".into(),
                        email: "lea@example-studio.com".into(),
                        photo: Some(bad),
                    },
                )
                .is_err()
        );
    }
    let me: MemberView = lea
        .client
        .call(
            "/v1/me",
            &ProfileChange {
                name: "Léa Martin".into(),
                email: "lea@example-studio.com".into(),
                photo: Some(String::new()),
            },
        )
        .unwrap();
    assert!(me.photo.is_none());
    let devs: Vec<DeviceView> = hugo.client.call("/v1/me/devices", &()).unwrap();
    assert!(devs.iter().any(|d| d.current && d.device == "PC"));
    assert!(
        devs.iter().any(|d| d.remembered_until.is_some()),
        "{devs:?}"
    );
    let _: PushReply = lea
        .client
        .call(
            "/v1/spaces/update",
            &SpaceUpdate {
                id: general,
                sealed: seal(&k1, b"space", b"General 2").unwrap(),
            },
        )
        .unwrap();
    let tmp = SpaceId::new();
    let _: PushReply = lea
        .client
        .call(
            "/v1/spaces",
            &NewSpace {
                id: tmp,
                sealed: vec![1],
                mode: SpaceMode::EndToEnd,
                reserved: false,
                wrapped: wrap_key(&lea.account.public(), &k1, tmp.0.as_bytes()).unwrap(),
                org_wrapped: None,
                recovery_wrapped: None,
            },
        )
        .unwrap();
    let _: PushReply = lea
        .client
        .call("/v1/spaces/delete", &IdOnly { id: tmp.0 })
        .unwrap();
    assert!(lea.state().spaces.iter().all(|s| s.id != tmp));
    let _: PushReply = hugo
        .client
        .call(
            "/v1/me/devices/sign-out",
            &DeviceRef {
                device: "PC".into(),
            },
        )
        .unwrap();
    assert_eq!(
        hugo.client.get::<State>("/v1/state").unwrap_err().code(),
        Some("unauthorized")
    );
    hugo.sign_in(Some(totp.code_at(now + 30))).ok();

    // Hugo is revoked: his session ends, General is marked for re-encryption.
    let _: PushReply = lea
        .client
        .call("/v1/members/revoke", &IdOnly { id: hugo.member.0 })
        .unwrap();
    assert_eq!(
        hugo.client.get::<State>("/v1/state").unwrap_err().code(),
        Some("unauthorized")
    );
    assert_eq!(hugo.sign_in(None).unwrap_err().code(), Some("revoked"));
    let st = lea.state();
    assert!(st.spaces[0].needs_rekey);
    assert_eq!(st.members.len(), 1);

    // Re-encryption: every row under a new key, generation 2; old-key writes refused after.
    let k2 = SymmetricKey::generate().unwrap();
    let ch: Changes = lea
        .client
        .call(
            "/v1/pull",
            &Pull {
                since: 0,
                full: vec![general],
            },
        )
        .unwrap();
    let rows: Vec<Row> = ch
        .rows
        .iter()
        .map(|r| row(general, &k2, &r.id, "re-sealed", r.changed, 2))
        .collect();
    let _: PushReply = lea
        .client
        .call(
            "/v1/spaces/rekey",
            &Rekey {
                space: general,
                key_gen: 2,
                sealed: seal(&k2, b"space", b"General").unwrap(),
                mode: SpaceMode::EndToEnd,
                wraps: vec![Wrap {
                    space: general,
                    member: lea.member,
                    wrapped: wrap_key(&lea.account.public(), &k2, general.0.as_bytes()).unwrap(),
                }],
                org_wrapped: None,
                recovery_wrapped: None,
                rows,
            },
        )
        .unwrap();
    let st = lea.state();
    assert!(!st.spaces[0].needs_rekey);
    assert_eq!(st.spaces[0].key_gen, 2);
    assert_eq!(st.spaces[0].mode, SpaceMode::EndToEnd);
    let r: PushReply = lea
        .client
        .call(
            "/v1/push",
            &Changes {
                rows: vec![row(general, &k1, "c", "old key", 9000, 1)],
                purged: vec![],
                seq: 0,
            },
        )
        .unwrap();
    assert_eq!(r.refused.len(), 1);
    // Erase for good.
    let _: PushReply = lea
        .client
        .call(
            "/v1/push",
            &Changes {
                rows: vec![],
                purged: vec![("entry".into(), "a".into())],
                seq: 0,
            },
        )
        .unwrap();
    let ch: Changes = lea
        .client
        .call(
            "/v1/pull",
            &Pull {
                since: st.seq,
                full: vec![],
            },
        )
        .unwrap();
    assert_eq!(ch.purged, vec![("entry".to_owned(), "a".to_owned())]);

    // Backup and restore of the server file.
    let backup = lea.client.send("GET", "/v1/backup", None).unwrap();
    assert_eq!(backup.status, 200);
    assert!(backup.body.starts_with(b"SQLite format 3"));
    let mut admin_rest = lea.client.clone();
    admin_rest.timeout = std::time::Duration::from_secs(30);
    let r = admin_rest
        .send("POST", "/v1/restore", Some(&backup.body))
        .unwrap();
    assert_eq!(r.status, 200, "{}", String::from_utf8_lossy(&r.body));
    lea.sign_in(None).unwrap();
    assert_eq!(lea.state().spaces[0].key_gen, 2);
    // Audit S-18: a copy whose activity log was changed is refused.
    let tampered = {
        let f = tempdir().join("copy.db");
        std::fs::write(&f, &backup.body).unwrap();
        let c = cleis_server::db::open(&f).unwrap();
        c.execute_batch(
            "DELETE FROM log WHERE seq = (SELECT min(seq) + 1 FROM log);
             PRAGMA wal_checkpoint(TRUNCATE);",
        )
        .unwrap();
        drop(c);
        std::fs::read(&f).unwrap()
    };
    let r = admin_rest
        .send("POST", "/v1/restore", Some(&tampered))
        .unwrap();
    assert_eq!(r.status, 400, "{}", String::from_utf8_lossy(&r.body));
    lea.sign_in(None).unwrap();

    // Key rotation: announced, then in use after the switch; apps that stored both still connect.
    let info: OrgInfo = lea.client.call("/v1/rotate", &()).unwrap();
    let next: [u8; 32] = info.next_cert.unwrap().try_into().unwrap();
    let dir = srv.stop();
    {
        let conn = cleis_server::db::open(&dir.join(cleis_server::DB_FILE)).unwrap();
        let c = cleis_server::db::meta_get(&conn, "next_cert")
            .unwrap()
            .unwrap();
        let k = cleis_server::db::meta_get(&conn, "next_cert_key")
            .unwrap()
            .unwrap();
        cleis_server::finish_rotation(&conn, &c, &k).unwrap();
    }
    let srv = Server::start(dir);
    let mut lea2 = Client::new(&srv.addr, Trust::Pinned(vec![info.cert]));
    assert!(matches!(
        lea2.get::<Hello>("/v1/hello"),
        Err(NetError::Untrusted)
    ));
    lea2.trust = Trust::Pinned(vec![info.cert, next]);
    lea.client = lea2;
    lea.sign_in(None).unwrap();
    assert!(lea.state().org.next_cert.is_none());
    srv.stop();
}

fn token(n: usize) -> String {
    include_str!("fixtures/oidc-tokens.txt")
        .lines()
        .find_map(|l| l.strip_prefix(&format!("T{n} ")))
        .unwrap()
        .to_owned()
}

/// Second steps by e-mail, SMS and an identity provider (D-64), codes kept by the server instead
/// of being sent, the provider's keys given beforehand.
#[test]
fn second_steps() {
    let dir = tempdir();
    let code = App::init(&dir, "Example Studio", &[]).unwrap();
    std::fs::write(
        dir.join("cleis-server.toml"),
        r#"
[smtp]
host = "smtp.example-studio.com"
from = "Cleis <vault@example-studio.com>"
[sms]
url = "https://sms.example-gateway.com/send"
body = "{to} {code}"
[[oidc]]
name = "google"
issuer = "https://idp.example-idp.com"
client_id = "cleis-test"
domains = ["example-studio.com"]
# The fixed test tokens were issued on 25/11/2026.
max_token_age = 3_000_000_000
"#,
    )
    .unwrap();
    let app = App::open(&dir).unwrap();
    app.capture_codes();
    app.keys.put(
        "https://idp.example-idp.com",
        vec![cleis_server::oidc::Jwk {
            kid: "k1".into(),
            kty: "RSA".into(),
            n: "ofgWCuLjybRlzo0tZWJjNiuSfb4p4fAkd_wWJcyQoTbji9k0l8W26mPddxHmfHQp-Vaw-4qPCJrcS2mJPMEzP1Pt0Bm4d4QlL-yRT-SFd2lZS-pCgNMsD1W_YpRPEwOWvG6b32690r2jZ47soMZo9wGzjb_7OMg0LOL-bSf63kpaSHSXndS5z5rexMdbBYUsLA9e-KXBdQOS-UTo7WTBEMa2R2CapHg665xsmtdVMTBQY4uDZlxvb3qCo5ZwKh9kG4LT6_I5IhlJH7aGhyxXFvUK-DWNmoudF8NAco9_h9iaGNj8q2ethFkMLs91kzk2PAcDTW9gb54h4FRWyuXpoQ".into(),
            e: "AQAB".into(),
        }],
    );
    let srv = Server::start_app(app.clone(), dir);
    let mut lea = join(&srv.addr, &code, "Lea Martin", "lea@example-studio.com").unwrap();
    lea.sign_in(None).unwrap();
    let st = lea.state();
    assert_eq!(st.methods.len(), 4);
    assert_eq!(st.providers[0].method, Method::Google);

    // Audit L-17: with [smtp] set up, an invitation goes by e-mail, and again on Resend.
    let inv: InviteReply = lea
        .client
        .call(
            "/v1/invites",
            &NewInvite {
                name: "Noa".into(),
                email: "noa@example-studio.com".into(),
                role: "member".into(),
                spaces: vec![],
                address: "vault.example-studio.lan:8443".into(),
                secret: None,
            },
        )
        .unwrap();
    assert!(inv.emailed);
    let again: InviteReply = lea
        .client
        .call(
            "/v1/invites/renew",
            &RenewInvite {
                id: inv.id,
                address: "vault.example-studio.lan:8443".into(),
                secret: None,
            },
        )
        .unwrap();
    assert!(again.emailed);
    let sent = app.sent_codes();
    for c in [&inv.code, &again.code] {
        assert!(sent.iter().any(|s| s.code == *c
            && s.to == cleis_server::delivery::To::Email("noa@example-studio.com".into())));
    }

    // Not allowed by the policy yet.
    let start = |m: Method, target: &str| CodeStart {
        method: m,
        target: target.into(),
    };
    let e = lea
        .client
        .call::<_, CodeSetup>("/v1/me/code", &start(Method::Email, ""))
        .unwrap_err();
    assert_eq!(e.code(), Some("forbidden"));
    let hello: Hello = lea.client.get("/v1/hello").unwrap();
    assert_eq!(hello.methods, vec![Method::AuthenticatorApp]);
    let mut policy = st.policy;
    policy.methods = Method::ALL.into_iter().collect();
    let _: Policy = lea.client.call("/v1/policy", &policy).unwrap();
    let hello: Hello = lea.client.get("/v1/hello").unwrap();
    assert_eq!(hello.methods.len(), 4);
    let last = || app.sent_codes().last().cloned().unwrap();

    // E-mail: set up, then each sign-in sends a code (one message a minute at most).
    let setup: CodeSetup = lea
        .client
        .call("/v1/me/code", &start(Method::Email, ""))
        .unwrap();
    assert_eq!(setup.sent_to, "l•••@example-studio.com");
    let sent = last();
    assert_eq!(
        sent.to,
        cleis_server::delivery::To::Email("lea@example-studio.com".into())
    );
    let wrong = if sent.code == "000000" {
        "111111"
    } else {
        "000000"
    };
    assert_eq!(
        lea.client
            .call::<_, SessionReply>("/v1/me/code/confirm", &CodeCheck { code: wrong.into() })
            .unwrap_err()
            .code(),
        Some("wrong_code")
    );
    let _: SessionReply = lea
        .client
        .call("/v1/me/code/confirm", &CodeCheck { code: sent.code })
        .unwrap();
    assert_eq!(lea.state().me.factor, Some(Method::Email));
    let e = lea.sign_in(None).unwrap_err();
    assert_eq!(e.code(), Some("code_sent"));
    let n = app.sent_codes().len();
    assert_eq!(lea.sign_in(None).unwrap_err().code(), Some("code_sent"));
    assert_eq!(
        app.sent_codes().len(),
        n,
        "no second message within a minute"
    );
    let c = last().code;
    lea.sign_in(Some(c.clone())).unwrap();
    assert_eq!(lea.sign_in(Some(c)).unwrap_err().code(), Some("wrong_code"));

    // SMS.
    let e = lea
        .client
        .call::<_, CodeSetup>("/v1/me/code", &start(Method::Sms, "12"))
        .unwrap_err();
    assert_eq!(e.code(), Some("bad_request"));
    let setup: CodeSetup = lea
        .client
        .call("/v1/me/code", &start(Method::Sms, "+33 6 99 00 00 12"))
        .unwrap();
    assert_eq!(setup.sent_to, "•••• 00 12");
    let _: SessionReply = lea
        .client
        .call("/v1/me/code/confirm", &CodeCheck { code: last().code })
        .unwrap();
    assert_eq!(lea.sign_in(None).unwrap_err().code(), Some("code_sent"));
    assert_eq!(
        last().to,
        cleis_server::delivery::To::Phone("+33699000012".into())
    );
    lea.sign_in(Some(last().code)).unwrap();

    // Google: bound at set-up, each token used once, another account refused.
    let setup: CodeSetup = lea
        .client
        .call("/v1/me/code", &start(Method::Google, &token(1)))
        .unwrap();
    assert!(setup.done);
    assert_eq!(lea.state().me.factor, Some(Method::Google));
    assert_eq!(
        lea.sign_in(None).unwrap_err().code(),
        Some("identity_google")
    );
    assert_eq!(
        lea.sign_in_with(None, Some(token(1))).unwrap_err().code(),
        Some("wrong_code"),
        "token used at set-up"
    );
    lea.sign_in_with(None, Some(token(2))).unwrap();
    assert_eq!(
        lea.sign_in_with(None, Some(token(2))).unwrap_err().code(),
        Some("wrong_code")
    );
    assert_eq!(
        lea.sign_in_with(None, Some(token(3))).unwrap_err().code(),
        Some("wrong_code"),
        "other domain"
    );
    srv.stop();
}

/// D-67 and audit S-1: what an Admin may do alone, what a Super Admin delegates.
#[test]
fn admin_delegations() {
    let dir = tempdir();
    let code = App::init(&dir, "Example Studio", &[]).unwrap();
    let srv = Server::start(dir);
    let mut lea = join(&srv.addr, &code, "Lea", "lea@example-studio.com").unwrap();
    lea.sign_in(None).unwrap();
    let inv = |p: &Person, email: &str, role: &str| -> Result<InviteReply, NetError> {
        p.client.call(
            "/v1/invites",
            &NewInvite {
                name: "X".into(),
                email: email.into(),
                role: role.into(),
                spaces: vec![],
                address: String::new(),
                secret: None,
            },
        )
    };
    let tom_code = inv(&lea, "tom@example-studio.com", ADMIN).unwrap();
    let mut tom = join(&srv.addr, &tom_code.code, "", "").unwrap();
    tom.sign_in(None).unwrap();

    // S-1: a Super Admin's invitation for an admin is neither seen, renewed nor revoked by an
    // Admin.
    let noa = inv(&lea, "noa@example-studio.com", SUPER_ADMIN).unwrap();
    assert!(!tom.state().invites.iter().any(|i| i.id == noa.id));
    let r: Result<InviteReply, NetError> =
        tom.client.call("/v1/invites/renew", &IdOnly { id: noa.id });
    assert_eq!(r.unwrap_err().code(), Some("forbidden"));
    let r: Result<PushReply, NetError> = tom
        .client
        .call("/v1/invites/revoke", &IdOnly { id: noa.id });
    assert_eq!(r.unwrap_err().code(), Some("forbidden"));
    let plain = inv(&tom, "sam@example-studio.com", "member").unwrap();
    let _: InviteReply = tom
        .client
        .call("/v1/invites/renew", &IdOnly { id: plain.id })
        .unwrap();

    // Not delegated: backups, keys, sign-in and log rules.
    let b = tom.client.send("GET", "/v1/backup", None).unwrap();
    assert_eq!(b.status, 403);
    let r: Result<OrgInfo, NetError> = tom.client.call("/v1/rotate", &());
    assert_eq!(r.unwrap_err().code(), Some("forbidden"));
    let mut p = tom.state().policy;
    p.record_activity = false;
    let r: Result<Policy, NetError> = tom.client.call("/v1/policy", &p);
    assert_eq!(r.unwrap_err().code(), Some("forbidden"));
    // The rest of the policy stays his.
    let mut p = tom.state().policy;
    p.min_length = 16;
    let _: Policy = tom.client.call("/v1/policy", &p).unwrap();
    // The Admin cannot delegate to himself.
    let mut roles = tom.state().roles;
    roles[1].perms.insert(Perm::Backups);
    let r: Result<Vec<Role>, NetError> = tom.client.call("/v1/roles", &roles);
    assert!(r.is_err());

    // Lea delegates backups and rules, takes the log back.
    let mut roles = lea.state().roles;
    roles[1].perms.insert(Perm::Backups);
    roles[1].perms.insert(Perm::Rules);
    roles[1].perms.remove(&Perm::Audit);
    let _: Vec<Role> = lea.client.call("/v1/roles", &roles).unwrap();
    let b = tom.client.send("GET", "/v1/backup", None).unwrap();
    assert_eq!(b.status, 200);
    let mut p = tom.state().policy;
    p.record_activity = false;
    let _: Policy = tom.client.call("/v1/policy", &p).unwrap();
    let r: Result<LogPage, NetError> = tom.client.call(
        "/v1/log/query",
        &LogQuery {
            member: None,
            action: None,
            space: None,
            after: None,
            before: None,
            limit: 10,
        },
    );
    assert_eq!(r.unwrap_err().code(), Some("forbidden"));
    srv.stop();
}

/// D-79: the organization's PIN. Set by an admin, asked of the others at sign-in and again
/// once "Ask again after" has passed; wrong PINs lock the member out until an admin unlocks.
#[test]
fn organization_pin() {
    let dir = tempdir();
    let code = App::init(&dir, "Example Studio", &[]).unwrap();
    let app = App::open(&dir).unwrap();
    let srv = Server::start_app(app.clone(), dir);
    let mut lea = join(&srv.addr, &code, "Lea", "lea@example-studio.com").unwrap();
    lea.sign_in(None).unwrap();
    let noa_code: InviteReply = lea
        .client
        .call(
            "/v1/invites",
            &NewInvite {
                name: "Noa".into(),
                email: "noa@example-studio.com".into(),
                role: "member".into(),
                spaces: vec![],
                address: String::new(),
                secret: None,
            },
        )
        .unwrap();
    let mut noa = join(&srv.addr, &noa_code.code, "Noa", "noa@example-studio.com").unwrap();

    let mut p = lea.state().policy;
    p.access = AccessRule {
        on: true,
        with: AccessWith::Pin,
        length: 4,
        again_hours: 8,
        idle_mins: None,
        tries: 3,
    };
    let _: Policy = lea.client.call("/v1/policy", &p).unwrap();
    // Only a digest comes (D-98); an unreadable one is refused.
    let r: Result<PushReply, NetError> = lea.client.call(
        "/v1/policy/access",
        &AccessSecret {
            salt: "00".into(),
            hash: "zz".into(),
            params: cleis_crypto::KdfParams::default(),
        },
    );
    assert_eq!(r.unwrap_err().code(), Some("bad_request"));
    let _: PushReply = lea
        .client
        .call(
            "/v1/policy/access",
            &cleis_server::new_access_secret("4821").unwrap(),
        )
        .unwrap();
    // The admin who set it goes on; the member is asked it.
    lea.state();
    // The refusal names the PIN's length: the app shows that many boxes.
    match noa.sign_in(None).unwrap_err() {
        NetError::Api(_, a) => {
            assert_eq!((a.error.as_str(), a.pin_length), ("pin_required", Some(4)))
        }
        e => panic!("{e}"),
    }
    noa.pin = Some("0000".into());
    let e = noa.sign_in(None).unwrap_err();
    assert_eq!(e.code(), Some("wrong_pin"));
    noa.pin = Some("4821".into());
    noa.sign_in(None).unwrap();
    noa.state();

    // "Ask again after" passed on this device: the session asks the PIN again.
    {
        let conn = app.db.lock().unwrap();
        conn.execute(
            "UPDATE pin_ok SET expires = 0 WHERE member = ?1",
            [noa.member.to_string()],
        )
        .unwrap();
    }
    let r: Result<State, NetError> = noa.client.get("/v1/state");
    assert_eq!(r.unwrap_err().code(), Some("pin_required"));

    // Three wrong PINs in a row: locked out, even with the right one.
    noa.pin = Some("1111".into());
    assert_eq!(noa.sign_in(None).unwrap_err().code(), Some("wrong_pin"));
    assert_eq!(noa.sign_in(None).unwrap_err().code(), Some("wrong_pin"));
    assert_eq!(noa.sign_in(None).unwrap_err().code(), Some("pin_locked"));
    noa.pin = Some("4821".into());
    assert_eq!(noa.sign_in(None).unwrap_err().code(), Some("pin_locked"));
    let st = lea.state();
    assert!(
        st.members
            .iter()
            .find(|m| m.id == noa.member)
            .unwrap()
            .pin_locked
    );
    // An admin unlocks.
    let _: PushReply = lea
        .client
        .call("/v1/members/reset-pin", &IdOnly { id: noa.member.0 })
        .unwrap();
    noa.sign_in(None).unwrap();
    assert!(
        !lea.state()
            .members
            .iter()
            .find(|m| m.id == noa.member)
            .unwrap()
            .pin_locked
    );
    // The admin's unlock is dated for the computers of a shared folder (D-98).
    assert!(
        lea.state()
            .members
            .iter()
            .find(|m| m.id == noa.member)
            .unwrap()
            .pin_reset
            .is_some()
    );
    // Typed again after inactivity (D-98): checked even before "Ask again after", and counted.
    let r: Result<PushReply, NetError> = noa.client.call(
        "/v1/access/check",
        &AccessCheck {
            secret: "9999".into(),
        },
    );
    assert_eq!(r.unwrap_err().code(), Some("wrong_pin"));
    let _: PushReply = noa
        .client
        .call(
            "/v1/access/check",
            &AccessCheck {
                secret: "4821".into(),
            },
        )
        .unwrap();
    // A member cannot change it.
    let r: Result<PushReply, NetError> = noa.client.call(
        "/v1/policy/access",
        &cleis_server::new_access_secret("1234").unwrap(),
    );
    assert_eq!(r.unwrap_err().code(), Some("forbidden"));
    // A password in place of the PIN (D-98): the old PIN no longer counts, the refusal says a
    // password is asked.
    p.access.with = AccessWith::Password;
    let _: Policy = lea.client.call("/v1/policy", &p).unwrap();
    assert!(!lea.state().pin_set);
    let _: PushReply = lea
        .client
        .call(
            "/v1/policy/access",
            &cleis_server::new_access_secret("a long organization password").unwrap(),
        )
        .unwrap();
    noa.pin = None;
    match noa.sign_in(None).unwrap_err() {
        NetError::Api(_, a) => {
            assert_eq!(a.error, "pin_required");
            assert!(a.password && a.pin_length.is_none());
        }
        e => panic!("{e}"),
    }
    noa.pin = Some("a long organization password".into());
    noa.sign_in(None).unwrap();
    // Windows Hello: checked by the member's computer, the server asks nothing.
    p.access.with = AccessWith::Hello;
    let _: Policy = lea.client.call("/v1/policy", &p).unwrap();
    noa.pin = None;
    noa.sign_in(None).unwrap();
    // The state never carries the digest from a server.
    assert!(noa.state().access_secret.is_none());
    srv.stop();
}

/// D-98: the logo, set by an admin, seen by the members; the Super Admin deletes the
/// organization, and every request then answers `deleted`.
#[test]
fn logo_and_deletion() {
    let dir = tempdir();
    let code = App::init(&dir, "Example Studio", &[]).unwrap();
    let app = App::open(&dir).unwrap();
    let srv = Server::start_app(app.clone(), dir);
    let mut lea = join(&srv.addr, &code, "Lea", "lea@example-studio.com").unwrap();
    lea.sign_in(None).unwrap();
    let png = [b"\x89PNG\r\n\x1a\n".as_slice(), &[0; 32]].concat();
    let _: PushReply = lea
        .client
        .call(
            "/v1/org/logo",
            &LogoSet {
                png: Some(png.clone()),
            },
        )
        .unwrap();
    assert_eq!(lea.state().org.logo.as_deref(), Some(png.as_slice()));
    let r: Result<PushReply, NetError> = lea.client.call(
        "/v1/org/logo",
        &LogoSet {
            png: Some(b"GIF89a".to_vec()),
        },
    );
    assert_eq!(r.unwrap_err().code(), Some("bad_request"));
    // The name typed again.
    let r: Result<PushReply, NetError> = lea.client.call(
        "/v1/org/delete",
        &OrgDelete {
            name: "Example".into(),
        },
    );
    assert_eq!(r.unwrap_err().code(), Some("bad_request"));
    let _: PushReply = lea
        .client
        .call(
            "/v1/org/delete",
            &OrgDelete {
                name: "Example Studio".into(),
            },
        )
        .unwrap();
    let r: Result<State, NetError> = lea.client.get("/v1/state");
    assert_eq!(r.unwrap_err().code(), Some("deleted"));
    assert_eq!(lea.sign_in(None).unwrap_err().code(), Some("deleted"));
    // Nothing left but the name and the mark.
    let conn = app.db.lock().unwrap();
    let n: i64 = conn
        .query_row("SELECT count(*) FROM members", [], |r| r.get(0))
        .unwrap();
    assert_eq!(n, 0);
    drop(conn);
    srv.stop();
}
