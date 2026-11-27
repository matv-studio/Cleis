# Cleis — état

## 27/11/2026 (version 0.2.3, branche `feat/full-app`)
- Retours sur la 0.2.2 (D-81 à D-87) : accent bronze, « New password », cadenas ouvert, survol des onglets,
  réglages sans phrases ; maquettes 37a à 37o et 38 validées et codées : dossier nommé sur place, traits
  entre en-têtes, point de connexion, Start in par onglet, raccourcis par coffre en un seul onglet, menu du
  champ mot de passe, suppression des sauvegardes et du compte, PIN du coffre (Argon2id + DPAPI),
  connexion GitHub par code (application OAuth « Cleis »), Paramètres › Shortcuts.
- Constat : le mot de passe maître et les éléments vus après l'installation de la 0.2.2 étaient le coffre
  d'essai du PC (`%APPDATA%\matv-studio\Cleis\data`), l'installateur n'en porte aucun.
- Essais : 194 réussis (dont 9 nouveaux de bout en bout), clippy, fmt.
- À essayer sur Windows : PIN (DPAPI réel), Windows Hello, connexion GitHub réelle (code sur
  github.com/login/device), clic droit sur le champ mot de passe.
- Suite du 27/11 (D-88 à D-90, maquettes 37p à 37r) : clic droit sans ouverture (ligne cernée), PIN de
  l'organisation en cases (longueur envoyée par le serveur, saisie admin dans le dialogue 37f), trousseau
  macOS pour le PIN et « Rien », GitHub sans coffre au premier lancement (« Create my vault »). Docs et
  builds rangés aussi dans `D:\.PRJCT_Perso\Dev\Cleis\` (pont du bureau : écriture seulement, pas de
  déplacement ni suppression).
- Soir du 27/11 (D-91 à D-95, maquettes 39 et 40 validées) : retour / avance à la souris, page
  « Account » unique (Security et Profile repliés, 2FA, Revoke access, Delete account), « Password »,
  premier écran en deux temps (options seules puis « Unlock with » ; GitHub sans coffre → création
  directe, avec coffre → ouverture sur la même carte ; le dialogue 37r est retiré), pastille du compte en
  haut à droite, un seul bloc Sync, onglet du coffre personnel seulement avec une organisation, survols
  des boutons visibles avec le bronze.
- Fin du 27/11 (D-96, D-97) : sans mot de passe, plus de clé de récupération imposée ; la clé du coffre est
  gardée dans le compte (`vault.key`) et un nouvel ordinateur ouvre le coffre en se connectant ; premier
  écran : organisation → « Unlock with » après l'invitation vérifiée, tête « Sign in » sans survol.
  GitHub App décidée (maquettes 41a-41b envoyées), à coder quand l'application existe.
- En attente de l'utilisateur : suppression du dépôt `matv-studio/cleis-vault` (la session ne peut pas) ;
  GitHub App à créer (Client ID + nom court) ; applications Microsoft (Entra) et Google (Cloud) à enregistrer, Client
  IDs à fournir ; la connexion Microsoft / Google (coffre dans le dossier d'appli OneDrive, dossier
  caché appDataFolder de Google Drive) se code ensuite sur le même parcours.
- Notes pour plus tard (demandes du 27/11) : calendrier pour gestionnaires de prod et employés (tâches,
  disponibilités) synchronisé avec Outlook (adresses et calendriers Outlook → Cleis) ; section « pages
  privées » pour construire pages et notes comme Notion ; changelog en anglais seulement sur les brouillons.

## 26/11/2026 (suite : version 0.2.2 en cours, branche `feat/full-app`)
- v0.2.1 : brouillon prêt (étiquette posée par Actions, D-77), à publier par l'utilisateur.
- Fait : coin de l'onglet actif d'un seul tracé, survol des onglets, sens de l'animation des
  onglets, transition de dossier, logo clair partout + réglage, menu clic droit des contacts,
  alignements au pixel (hauteur de ligne 18 px, anneau d'accent 30 px), charte mise à jour
  (projet claude.ai, section 10), roue chromatique (36), photo d'un contact (36e, vCard PHOTO),
  code PIN d'organisation (36g/36h, D-79), création d'une organisation sur le NAS / cet ordinateur
  / un dossier partagé (36f, 36i, D-78) : `nas.rs` (SSH, QNAP .qpkg, systemd, simple),
  `packaging/qnap/` + CI (serveur musl x64 / arm64, paquets QPKG), `cleis_server::local` (API
  rejouée), `folder_org.rs` (opérations signées dans le dossier, copie locale).
- NAS de l'utilisateur : QNAP TS-863U-RP, AMD GX-420MC (x86-64), QTS 5.2.11, pas de Container
  Station : le paquet .qpkg (App Center) est la voie retenue, sans Docker.
- Aussi : SFTP comme dossier partagé (`folder_store.rs`), compte (36b–36d, D-80 : GitHub ;
  Microsoft / Google grisés jusqu'à l'enregistrement OAuth), installateur portant Cleis Server pour
  NAS (`server/`), version 0.2.2, notes `docs/notes/v0.2.2.md`.
- Étiquette v0.2.2 posée sur f2f79b3 (D-77) ; brouillon « Cleis v0.2.2 » construit (installateur,
  serveurs Linux, binaires et .qpkg pour NAS), à publier par l'utilisateur. Reste : essais réels
  (création sur le QNAP par SSH, dossier SharePoint).

## 26/11/2026 (version 0.2.1 en préparation, branche `feat/full-app`)
- v0.2.0 publiée par l'utilisateur. Mises à jour depuis GitHub (D-74), installateur Windows et une
  seule copie de Cleis (D-75), barre du haut propre à Cleis (Windows : contrôles cliquables).
- Lot A (retours d'essai) : erreurs lisibles, boutons de fenêtre à la charte, clic dans le vide qui
  désélectionne, « × » des panneaux, « Saved » 1,5 s, section « My vault ».
- Lot B (maquettes 35 à 35k validées) : barre latérale en sections, raccourcis par coffre ; menus
  du clic droit dans le vide ; tris dont « My order » ; glisser-déposer (élément → dossier, dossiers
  et éléments réordonnés) ; largeur des colonnes ; mode Personnaliser ; calendrier des dates ;
  ouverture de Cleis par mot de passe, Windows Hello ou rien, clé de récupération, règle de
  l'organisation (D-76) ; animations courtes (aucune si Windows les coupe).
- Nouveau logo (clair / sombre) : appli, exécutable, installateur, zone de notification.
- Essais : 21 + 28 de bout en bout + 4 organisation dans `cleis-app`, plus les crates.

## 25/11/2026 (organisation, branche `feat/full-app`)
- `cleis-server` (D-59) et `cleis-net` (client HTTPS épinglé) : invitation, connexion par clé de compte
  et code, droits par rôle, versions, journal signé et chaîné, révocation, rechiffrement, sauvegarde /
  restauration, rotation du certificat.
- Coffre : espaces d'organisation dans le fichier, hors des copies personnelles ; envoi / réception des
  lignes scellées, rechiffrement préparé puis appliqué.
- Appli : rejoindre au premier lancement ou par Settings › Organization ; pages Members, Security
  policy, Encryption, Server, sauvegardes de l'organisation ; onglet Activity ; dialogues invitation,
  code, nouveau rôle, nouvel espace, rechiffrement, code demandé par le serveur. Maquettes ajoutées :
  01c, 05b, 07b–d, 24, 25. Démo : organisation fictive (`--demo`, `--no-org` pour la page de jonction).
- Essais : protocole complet sur TLS réel (`crates/cleis-server/tests/flow.rs`) ; scénario à deux
  fenêtres admin / membre contre un vrai serveur (`crates/cleis-app/tests/org.rs`).

- Suite (D-60) : rôle Super Admin (transfert, ajout, au moins un), clés de sécurité retirées, pages
  Profile, Devices, Spaces (renommer, supprimer). Maquettes 07 mise à jour, 07e, 26–28. Captures en
  thème sombre seulement.

- Suite (D-61 à D-63) : espaces réservés (Super Admin), dossiers personnels, serveur universel
  (services Linux / Windows / macOS, Docker, configuration, sauvegardes automatiques), création
  d'une organisation depuis l'appli (serveur sur cet ordinateur ou commandes pour une autre machine).
  Maquettes 25b, 24b. Essai `organization_hosted_here` (lance le vrai `cleis-server`).

- D-64 : deuxième étape de connexion par e-mail, SMS, Google, Microsoft (serveur et appli, essais
  `second_steps` et `sms_second_step`).

- D-65 : places GitHub (jeton) et SFTP, restauration depuis elles, copies des sauvegardes de
  l'organisation.

- D-66 : déverrouillage par Windows Hello / Touch ID.

- Audit complet fait : `docs/AUDIT-2026-11-25-complet.md` (1 critique, 8 élevés, 28 moyens,
  31 faibles).

- Corrections (D-67, D-68) : droits délégués à l'Admin ; S-1, S-4, S-5, S-8, S-10, L-1 à L-5, L-7,
  L-8, L-12 ; menus contextuels (clic droit), raccourcis de l'élément, confirmations, Échap (U-1,
  U-2), nom « Two-step verification » (U-4). Maquettes 07f, 08b, 32–34b validées.

- D-69 : onglets de coffre (Preferences › Vaults), plusieurs organisations par coffre, filtre de
  coffre dans Health (maquette 32e). Build Windows : branche `build/full-app`.

- D-70 : fin de l'audit complet (sécurité, logique, interface, scripts, CI, docs), sauf cinq
  points à trancher par l'utilisateur (« En suspens » 10 à 14 de DECISIONS.md). Version 0.2.0
  préparée (pas d'étiquette : pas de version tant que l'audit n'est pas fini).

- D-71 à D-73 : choix de fin d'audit faits (biométrie retirée, sauvegardes du serveur en clair
  sans restriction, élagage hebdomadaire de la branche GitHub, fonctions non faites retirées) ;
  interface traduite en français (langue du système au premier lancement, réglable dans
  Preferences, captures `UI_LANG=fr design/tools/appshots.sh`). L'audit est terminé.

## Prochaines étapes
1. Version `v0.2.1` : build Windows vérifié sur `build/full-app`, installateur posé sur le PC de
   l'utilisateur pour essai ; script d'étiquette et notes EN + FR préparés dans
   `C:\MATV_LOCAL\_Interne\Cleis\`. L'utilisateur décide quand la pousser.
2. Essayer Windows Hello sur une vraie machine (D-76 : seulement compilé ici).
3. Discussion : une seule appli (bureau) sans serveur à installer.
4. Essais sur machines réelles : e-mail SMTP, passerelle SMS, clients OAuth Google / Microsoft.
5. Format monétaire régional en français (`69,99 €`, « En suspens » 15).

## Historique

### 25/11/2026 (suite, branche `feat/full-app`)
- Essais Windows en pilotage d'écran (builds `651fe04` → `3558b4d`) : Quick Access sur le bon écran et
  au clavier (plus d'Alt simulé : touche F22 factice comme Chromium), remplissage identifiant + mot de
  passe validé sur le formulaire d'essai (`design/tools/autofill-form.hta`, lancé par
  `ouvrir-formulaire.bat`), garde-fou : rien n'est tapé dans un bouton ou un menu, masque Alt (0xE8)
  après un raccourci. Icône de zone de notification : fermer cache, un clic rouvre.
- Fonctions ajoutées (D-54 à D-58) : menu Colonnes + champs personnalisés ; import CSV en 3 étapes ;
  filtre et vue Cartes du coffre ; contacts (édition, dossiers, étiquettes, Merge & fix, vCard) ;
  Sécurité (code à l'ouverture + codes de secours, verrouillage après inactivité, presse-papiers,
  mot de passe maître) ; synchronisation par dossier (OneDrive / Google Drive par leur appli, partage),
  sauvegardes datées, export, kit d'urgence PDF, restauration au premier lancement ; santé par espace
  avec corrections ; préférences (devise, vue de départ) ; icône de zone de notification et ouverture
  à la connexion.
- 101 essais (dont 12 de bout en bout).


### 25/11/2026
- Apparence : sombre, accent Ochre `#e0a93b` (D-51 ; teinte approfondie calculée en clair), étendue
  « Étendue » ; pastilles et étendue réglables dans *Preferences* ; menu « Sync & backups » unique.
- Audit avant build : `docs/AUDIT-2026-11-25.md` (corrections : NFC du mot de passe maître, création
  atomique, effacement des clés, générateur, formules bornées, imports Excel/Notion/Outlook).
- Remplissage (D-53) : crate `cleis-autofill` (raccourcis globaux, fenêtre au premier plan, adresse de
  l'onglet par UIA, remplissage UIA ou frappe, collage), Quick Access (écran 17), réglages Autofill
  complets (méthode, vitesse, raccourcis enregistrables, applis liées), Ctrl+L / Ctrl+F.
- 77 essais. Premier build Windows : workflow `release.yml` (portable zip, brouillon de version).


### 24/11/2026
- Maquettes : 21 écrans × 2 thèmes (coffre, colonnes, premier lancement, déverrouillage, 2FA, sécurité,
  membres, politique, chiffrement, rechiffrement, journal, santé, sauvegardes, import, préférences,
  remplissage, Quick Access, contacts, fusion des doublons). Validées sauf Contacts (20, 21).
- Code :
  - `cleis-crypto` : 12 tests (NIST AES-GCM, RFC 9106, 7748, 8032).
  - `cleis-core` : 27 tests (RFC 6238, 4226, vCard 2.1/3.0/4.0, import Notion, formules, journal
    signé et chaîné, doublons, santé, générateur).
  - `cleis-store` : 6 tests (aller-retour chiffré, aucun texte en clair dans le fichier ni le WAL,
    corbeille, changement de mot de passe, enregistrement déplacé rejeté).

- Appli GPUI (`cleis-app`) : premier lancement (création du coffre), déverrouillage, coffre (barre
  latérale, recherche, tableau virtualisé fidèle à la maquette 01, inspecteur, copie avec effacement du
  presse-papiers, générateur, favoris, nouvel élément), contacts (lecture), santé, réglages (Préférences :
  thème, onglets ; Sécurité : presse-papiers ; Remplissage : interrupteurs). 2 essais e2e.

