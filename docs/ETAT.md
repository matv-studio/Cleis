# Cleis — état

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

## Prochaines étapes
1. Retour de l'utilisateur sur le build.
2. Reste de l'audit : U-6, U-10 à U-12, S-6, S-7, S-12, L-6, L-10, L-13 à L-19, U-8, X-1 à X-9,
   D-3 à D-5.
3. Essais sur machines réelles : Windows Hello, Touch ID, e-mail SMTP, passerelle SMS, clients
   OAuth Google / Microsoft, serveur hébergé et service installé.


## 25/11/2026 (suite, branche `feat/full-app`)
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

## Prochaines étapes
1. Contacts : menu Colonnes et import CSV.
2. Fonctions d'organisation (écrans 07-11, 18) et `cleis-server` (D-42) : membres, partage des clés
   d'espace, politique, journal, rechiffrement.
3. Places GitHub / OneDrive / Google sans appli de bureau : enregistrement OAuth (en suspens, D-50).
4. Audit complet demandé par l'utilisateur une fois tout en place.

## 25/11/2026
- Apparence : sombre, accent Ochre `#e0a93b` (D-51 ; teinte approfondie calculée en clair), étendue
  « Étendue » ; pastilles et étendue réglables dans *Preferences* ; menu « Sync & backups » unique.
- Audit avant build : `docs/AUDIT-2026-11-25.md` (corrections : NFC du mot de passe maître, création
  atomique, effacement des clés, générateur, formules bornées, imports Excel/Notion/Outlook).
- Remplissage (D-53) : crate `cleis-autofill` (raccourcis globaux, fenêtre au premier plan, adresse de
  l'onglet par UIA, remplissage UIA ou frappe, collage), Quick Access (écran 17), réglages Autofill
  complets (méthode, vitesse, raccourcis enregistrables, applis liées), Ctrl+L / Ctrl+F.
- 77 essais. Premier build Windows : workflow `release.yml` (portable zip, brouillon de version).

## Prochaines étapes
1. Essai du build Windows en pilotage d'écran : Quick Access, Fill best match (Chrome, Edge, appli de
   bureau), formulaire en deux pages, 2FA ; corriger ce qui en sort.
2. Icône de zone de notification (tray-icon) : Cleis en arrière-plan, raccourcis actifs fenêtre fermée.
3. Import CSV / vCard branché (écran 14), export chiffré.
4. Édition dans l'inspecteur, menu Colonnes, corbeille, anneau TOTP dessiné.
5. Synchronisation et restauration (D-50, écrans 22, 23, après validation).
6. `cleis-server`.

## 24/11/2026
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

## Prochaines étapes
1. Édition dans l'inspecteur (coffre et contacts), menu Colonnes (affichage, ordre, colonnes perso),
   suppression et corbeille, anneau TOTP dessiné (canvas).
2. Import CSV et vCard branchés à l'interface ; export chiffré.
3. Remplissage : raccourci global, Quick Access, écriture par accessibilité (macOS AX, Windows UIA),
   frappe en repli.
4. `cleis-server` : synchronisation, membres, partage des clés d'espace, journal côté serveur.
5. Sauvegardes GitHub / SFTP.
