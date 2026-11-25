# Audit complet — 25/11/2026 (après D-66)

Revue interne de `feat/full-app` au commit `eaeced8`. Rien n'a été corrigé pendant l'audit.

Portée : serveur, chiffrement, appli (tous les écrans et menus), logique d'usage et entre outils,
charte de design, scripts, CI, déploiement, documentation.

Chaque constat porte sa preuve :
- **[essai]** : reproduit par un essai écrit pour l'audit (non gardé dans le dépôt) ;
- **[code]** : lu dans le code, non exécuté ;
- **[capture]** : vu sur les captures de l'appli (42 du script + 10 faites à la main) ;
- **[déduit]** : conséquence logique, à confirmer sur machine.

## 1. Résumé

| Gravité | Nombre | Les plus importants |
|---|---|---|
| Critique | 1 | Un Admin devient Super Admin en renouvelant une invitation (S-1) |
| Élevée | 8 | Modifications d'un membre en lecture seule gardées en local sans erreur (L-1) ; élément déplacé hors d'un espace, copies fantômes (L-2) ; six réglages de la politique sans effet (L-3) ; actions irréversibles en un clic (U-1) ; sauvegarde serveur lisible par tout Admin (S-2) |
| Moyenne | 28 | Échap et clic ailleurs ne ferment rien (U-2) ; synchronisation au verrouillage perdue pour GitHub / SFTP (L-5) ; service Windows depuis un dossier modifiable (S-6) ; Microsoft multi-locataire cassé (S-8) |
| Faible | 31 | Écarts de charte, noms multiples d'une même fonction, scripts, docs |

État des vérifications automatiques : format conforme, analyse (clippy) sans avertissement,
135 essais réussis, 0 vulnérabilité connue dans les dépendances (`cargo audit`).

## 2. Sécurité

### Critique

**S-1 Un Admin obtient le rôle Super Admin (ou un espace réservé). [essai]**
`/v1/invites/renew` ne demande que le droit « Manage ». Un Admin voit dans la liste l'invitation
qu'un Super Admin a faite pour un futur Super Admin, la renouvelle, reçoit un code neuf, rejoint
avec un second coffre : il est Super Admin. Même chemin pour une invitation qui donne un espace
réservé (l'accès garde « donné par un Super Admin », donc les applis remettent la clé).
`/v1/invites/revoke` a le même manque (un Admin annule l'invitation d'un Super Admin).
Annule la séparation Admin / Super Admin de D-60 et D-61.
Pistes : (a) appliquer à `renew` et `revoke` les mêmes règles qu'à la création (rôle admin ou
espace réservé → Super Admin seulement) ; (b) en plus, cacher ces invitations aux Admins.
(a) suffit à fermer la faille ; (b) évite aussi la fuite de l'existence de l'invitation.

### Élevée

**S-2 La sauvegarde du serveur donne à tout Admin la clé du serveur et les clés TOTP. [code]**
`/v1/backup` (droit « Manage ») renvoie le fichier entier : clé privée du certificat, clé de
signature du journal, clé d'authentificateur de chaque membre (en clair dans la table), qui a
accès à quel espace réservé. Un Admin technique peut donc : se faire passer pour le serveur,
fabriquer un journal valide, produire les codes TOTP d'un Super Admin, connaître les membres d'un
espace réservé. Le contenu des espaces reste scellé.
Pistes : (a) réserver la sauvegarde au Super Admin ; (b) sceller dans la base les clés TOTP et la
clé du journal avec une clé hors base ; (c) exclure ces colonnes de la copie remise aux Admins.
(a) est le plus court mais retire la sauvegarde à l'Admin technique ; (b) et (c) la lui laissent.

**S-3 Un Admin règle seul la politique de sécurité. [essai]**
« Manage » suffit pour rendre le code facultatif, couper le journal d'activité, réduire sa durée.
Le changement est inscrit au journal, mais il rend possible le reste sans trace. À trancher :
politique réservée au Super Admin, ou seulement ses parties sensibles (journal, deuxième étape).

**S-4 Requêtes sans limite avant connexion. [code]**
Limite de corps de 256 Mo appliquée aussi à `/v1/join` et `/v1/session` (sans connexion) ; pas de
délai de lecture par connexion ; pas de plafond de connexions ; la table des échecs par adresse
ne se vide jamais. Un poste du réseau peut saturer la mémoire du serveur. Plus sensible depuis
D-63 : écoute par défaut sur toutes les interfaces (D-32 disait « interface VPN seulement »).

### Moyenne

- **S-5 Verrou empoisonné. [code]** 49 `expect("database lock")` : une panique dans une requête
  rend toutes les suivantes en erreur jusqu'au redémarrage. Aucune panique atteignable n'a été
  trouvée, mais rien ne la contient.
- **S-6 Service Windows lancé depuis un dossier que l'utilisateur peut modifier. [déduit]**
  « Install » (page Server) inscrit en service système (compte LocalSystem) le `cleis-server.exe`
  posé à côté de l'appli (zip portable, souvent dans Téléchargements). Remplacer ce fichier donne
  les droits système ; déplacer le dossier casse le service. Pistes : copier l'exécutable dans
  `Program Files` avant d'installer, ou faire tourner le service sous un compte restreint.
- **S-7 Dossier de données Windows lisible par les autres comptes. [déduit]** Sous Unix le dossier
  est en 0700 ; sous Windows aucun droit n'est posé sur `%ProgramData%\Cleis Server` (lisible par
  défaut par tous les utilisateurs du poste) : clé du serveur et clés TOTP exposées.
- **S-8 Microsoft « tout locataire » ne fonctionne pas. [code]** Le guide propose `{tenantid}`
  dans l'émetteur ; le serveur cherche alors ses clés à une adresse contenant littéralement
  `{tenantid}` : échec. De plus la restriction par domaine s'appuie sur l'adresse du jeton, que
  Microsoft ne garantit pas vérifiée : à ne pas utiliser comme contrôle en multi-locataire.
- **S-9 Windows Hello / Touch ID : un contrôle d'interface, pas un verrou cryptographique.
  [code]** La clé de l'appareil est gardée par DPAPI (Windows) ou le trousseau (macOS) : tout
  programme lancé sous le compte de l'utilisateur peut la lire sans passer par Hello. La
  vérification biométrique protège contre quelqu'un devant l'écran, pas contre un logiciel
  malveillant sous la session. C'est le niveau de plusieurs gestionnaires du marché, mais D-66 ne
  le dit pas. Piste plus forte sous Windows : clé liée à Hello (`KeyCredentialManager`).
- **S-10 Pas de remise à zéro de la deuxième étape. [code]** Téléphone perdu : aucun moyen pour un
  Super Admin de la retirer à un membre, pas de codes de secours côté serveur. Seule issue :
  retirer le membre et le réinviter. Changer de méthode ne retire pas les appareils mémorisés.
- **S-11 Sauvegardes automatiques du serveur en clair. [code]** `DATA/backups/*.db` contient les
  mêmes secrets que S-2 ; sans risque dans le dossier 0700, risqué si `dir` vise un partage réseau.
- **S-12 Paramètres Argon2 sans plancher. [code]** Déjà noté au premier audit, toujours ouvert.

### Faible

- S-13 Le jeton GitHub peut partir en clair vers `http://127.0.0.1` (exception faite pour
  l'essai, présente dans le code livré).
- S-14 Jeton « appareil mémorisé » en clair dans `<coffre>.org.json` (sans la clé du compte il
  ne sert à rien).
- S-15 Jeton d'identité : `nonce` vérifié par l'appli seulement, `azp` et `iat` non lus par le
  serveur (le jeton reste à usage unique).
- S-16 Un code par e-mail ou SMS peut être demandé une fois par minute par qui détient la clé du
  compte (coût des SMS).
- S-17 Actions GitHub épinglées par étiquette (`@v5`), pas par empreinte.
- S-18 `restore` ne vérifie ni la chaîne ni la signature du journal de la copie remise, et fait
  revenir les membres retirés depuis la sauvegarde (non signalé à l'écran).

## 3. Logique d'usage et logique entre outils

### Élevée

**L-1 Lecture seule : la modification reste en local, sans erreur. [essai]**
Un membre « Read only » modifie un mot de passe d'un espace partagé : son appli l'affiche comme
enregistré, le serveur le refuse, l'appli ignore le refus (`refused` n'est lu nulle part) et
marque la ligne comme envoyée. Lui voit sa valeur, les autres l'ancienne, indéfiniment. Même
chose pour une suppression. Les droits « Add, edit » et « Delete » ne désactivent aucun bouton.
Pistes : (a) griser l'édition selon le rôle ; (b) lire `refused`, rétablir la version du serveur
et le dire. (a) évite l'erreur, (b) couvre aussi les cas de course ; les deux se complètent.

**L-2 Déplacer un élément hors d'un espace partagé. [essai]**
- Vers un dossier personnel (« Move to My vault ») : l'élément reste sur le serveur et chez les
  autres ; celui qui l'a déplacé ne le voit plus dans l'espace, il en garde une copie privée. Rien
  au journal : c'est une sortie de données non tracée, et la colonne « Export » n'y change rien.
- Entre deux espaces partagés **[code]** : les membres qui n'ont que l'espace de départ ne
  reçoivent jamais l'information ; ils gardent l'ancienne copie.
Pistes : interdire org → personnel (ou le traiter comme un export journalisé avec suppression
côté serveur) ; pour org → org, envoyer un effacement aux membres de l'espace quitté.

**L-3 Six réglages de la politique de sécurité n'ont aucun effet. [code + capture]**
| Réglage affiché | Réalité |
|---|---|
| Offline access | durée fixée à 14 jours dans l'appli, valeur ignorée |
| Passwords › Flag shorter than / older than | le rapport Health utilise des valeurs par défaut |
| Autofill : fill fields / type keys / read addresses | lus par aucun code de l'appli |
Un admin croit avoir réglé une règle qui ne s'applique pas.

**L-4 Colonne « Export » de Members sans effet (D-19). [code]**
Aucun export des éléments d'organisation n'existe ; l'interrupteur ne garde qu'un événement que
l'appli n'envoie jamais. L'export vCard des contacts partagés n'est ni soumis à ce droit ni
journalisé. Le droit « Copy, fill » d'un rôle n'est pas appliqué non plus (seul « See » l'est).

### Moyenne

- **L-5 Synchronisation au verrouillage perdue pour GitHub / SFTP. [code]** Le verrouillage lance
  la synchronisation puis ferme le coffre aussitôt ; la tâche réseau revient sur un coffre fermé
  et s'arrête sans rien envoyer ni signaler. L'info-bulle promet « when it locks ».
- **L-6 Adresse du serveur non modifiable. [code]** Si l'ordinateur hôte change d'adresse (DHCP),
  chaque membre doit quitter l'organisation et être réinvité.
- **L-7 Kit d'urgence faux pour GitHub / SFTP. [code]** Il imprime le chemin de la copie locale
  (`…cleis.remote`) au lieu du dépôt ou du serveur : inutilisable le jour où il sert.
- **L-8 Secret de place partagé entre appareils. [code]** Le jeton GitHub / mot de passe SFTP est
  un réglage du coffre, donc synchronisé : « Disconnect » sur un appareil le retire aux autres,
  dont la synchronisation échoue ensuite.
- **L-9 GitHub : chaque synchronisation ajoute le coffre entier à l'historique du dépôt.
  [déduit]** Le dépôt grossit de la taille du coffre à chaque envoi. Non mesuré ; à suivre ou à
  borner (branche réécrite, ou envoi moins fréquent).
- **L-10 Organisation créée « sur cet ordinateur ». [code]** Le nom du membre est le nom de
  session du système, sans le demander, et l'e-mail est vide ; quitter l'organisation laisse le
  serveur tourner et ses données en place ; une seconde création part sur le port suivant.
- **L-11 Une seule organisation par coffre. [code]** Non dit dans l'interface.
- **L-12 Biométrie coupée par défaut en organisation. [code]** La politique « Biometrics » est
  désactivée par défaut : rejoindre une organisation retire Windows Hello à la synchronisation
  suivante, y compris pour le coffre personnel.

### Faible

- L-13 Groupe des dossiers de contacts partagés intitulé du nom du premier espace (« General »)
  au lieu du nom de l'organisation. [code + capture]
- L-14 Barre d'état « End-to-end encryption » affichée quand rien n'est sélectionné dans
  « All items », alors que la liste mêle les deux modes. [capture]
- L-15 Démo : le dialogue de rechiffrement montre « Accounting » (réservé) passant en Standard,
  ce que le serveur interdit ; le journal de démo montre un « Exported » qui n'existe pas.
- L-16 « Delete space » affirme « A backup of the organization keeps them » sans vérifier qu'une
  sauvegarde existe.
- L-17 Les invitations ne partent pas par e-mail même quand `[smtp]` est réglé ; « Resend »
  produit en fait un nouveau code à transmettre à la main.
- L-18 Preferences › Tabs ne propose pas de masquer Activity (D-40 le prévoit).
- L-19 Changer le mot de passe maître ne propose pas de retirer la clé biométrique de l'appareil.

## 4. Interface, menus et direction artistique

### Élevée

**U-1 Actions irréversibles en un clic, sans confirmation. [code]**
Remove (membre), Transfer Super Admin (on perd son propre rôle), Make Super Admin, Leave
(organisation), Rotate (clé du serveur), Revoke (invitation), Disconnect (place et copies de
sauvegarde). La charte demande une confirmation, focus sur « Cancel ». Seuls la corbeille, les
contacts, les dossiers et les espaces confirment.

### Moyenne

- **U-2 Échap et clic ailleurs. [code]** Échap ne ferme aucun dialogue ni menu de la fenêtre
  principale (seulement Quick Access et la saisie d'un raccourci). Sur 18 menus flottants, un seul
  (colonnes) se ferme au clic ailleurs. Charte § 7.
- **U-3 Clavier. [code]** Deux raccourcis en tout (Ctrl+L, Ctrl+F). Pas de nouveau (Ctrl+N), pas
  de flèches dans la liste, pas d'annulation (Ctrl+Z), pas de menu au clic droit. La charte
  demande « gestes d'abord, clavier partout » et un menu contextuel sur tout objet de liste.
- **U-4 Une même fonction, quatre noms. [capture]** « 2FA » (colonne Members), « Verification
  code » (Security policy, Devices), « Second step » (Profile), « verification » (dialogue). Et
  « Add an organization server » (premier lancement) contre « Join an organization » (réglages).
  Charte § 1.6 : un nom partout.
- **U-5 Deux « codes » sans lien visible. [capture]** Security › « Verification code on open »
  (local) et Profile › « Second step » (serveur) sont sur deux pages ; la maquette 06 les
  réunissait.
- **U-6 Export en double. [capture + code]** Import & export montre « Encrypted export » grisé
  (« Comes with Sync & backups ») alors que la fonction marche dans Sync & backups.
- **U-7 Espaces et dossiers : deux gestes. [capture]** Dossier personnel : « ⋯ » au survol dans la
  barre latérale. Espace d'organisation : Settings › Spaces, icône « T » pour renommer. Même
  objet pour l'utilisateur, deux endroits et deux gestes.
- **U-8 Maquette 23 périmée. [capture]** Elle montre « Devices », « Setup code », « Sign in »
  (OneDrive, Google) qui n'existent pas ; l'appli a « Choose folder ». La maquette fait foi selon
  la charte : à remettre d'accord dans un sens ou l'autre.
- **U-9 Langue. [code]** Tous les textes sont dans le code, en anglais seulement. La charte § 8
  demande des fichiers de traduction ; D-18 dit anglais. Contradiction à trancher.

### Faible (charte)

- U-10 Gris de texte anciens : `#9d9d9d` / `#8a8a8a` au lieu de `#a3a3a3` / `#969696` (charte du
  25/11). Le gris pâle sur fond de section donne 4,4:1, sous le seuil AA de 4,5.
- U-11 Voile des dialogues écrit en dur (`noir 45 %`) à cinq endroits ; le jeton existe (55 % en
  sombre, 28 % en clair). En thème clair le voile est trop sombre. [capture]
- U-12 Cinq copies du même dialogue (security, trash, places, org, folders), largeurs 400, 420,
  440, 480 (charte : 420).
- U-13 Accent ocre par défaut (D-51) alors que la charte dit « aucun » par défaut ; D-48 (bronze)
  n'est pas marquée remplacée.
- U-14 Phrases d'explication visibles dans des dialogues et sections (« Scan with an
  authenticator app… », « This vault stays on this computer », sous-titres de sections) ; la
  charte les veut en info-bulle.
- U-15 Sync & backups : le choix « Mine / Example Studio » est au-dessus du titre dans une vue, à
  côté dans l'autre.
- U-16 Deux tailles hors échelle (17 px), menus plus étroits que 220 px, 9 boutons-icônes sur 38
  sans info-bulle à proximité dans le code.
- U-17 Fonction prévue encore grisée : « Passwords found in breaches », extension de navigateur.

### Menus et sous-menus

Vus en capture : filtre, colonnes et nouvelle colonne, Quick Access, menu de dossier, menu
« ⋯ » d'un membre, invitation, code d'invitation, nouveau rôle, nouvel espace, renommer /
supprimer (espace, dossier), rechiffrement, code à l'ouverture, deuxième étape (4 variantes),
places GitHub et SFTP.

Relus dans le code seulement, faute d'option de capture : menu « ⋯ » de l'inspecteur (Duplicate,
Move to…, Move to trash), menus des contacts (vCard, Merge, Delete, libellés, liens), listes
After inactivity / Clear after copying / Keep, sélecteur de rôle et d'espaces d'un membre,
filtres du journal, tri des colonnes, vue en cartes, onglet History, corbeille, zone de
notification. Ces menus ne sont donc jamais contrôlés visuellement par le script.

## 5. Scripts, CI et déploiement

- **X-1 (moyenne) Notices de licence absentes des archives.** `THIRD_PARTY.md` dit que les textes
  de licence sont joints aux versions ; `release.yml` ne les joint pas (Apache-2.0 l'exige).
  L'archive macOS n'a même pas `THIRD_PARTY.md`.
- **X-2 (moyenne) La CI ne tourne pas sur la branche de travail.** Déclenchée sur `main` et les PR
  seulement ; avec l'envoi direct sur `feat/full-app`, chaque passage est lancé à la main.
- **X-3 (moyenne) Essais sautés en silence sur la CI.** L'essai SFTP (pas de serveur OpenSSH) et,
  selon l'ordre de construction, celui du serveur hébergé s'affichent « ok » sans avoir tourné.
- **X-4 (moyenne) Docker.** Pas de `.dockerignore` : `COPY . .` envoie `target/` (plus de 20 Go).
  Le contrôle de santé appelle `status`, qui lit le fichier sans interroger le serveur : toujours
  « sain ». Image jamais construite (pas de Docker dans le conteneur).
- X-5 `appshots.sh` : 9 vues récentes absentes ; attente fixe de 9 s par capture (15 min au
  total) ; une appli qui plante donne une capture vide sans erreur.
- X-6 `shots.py` : thèmes sombre + clair par défaut (consigne : sombre seulement) ; port 8180
  fixe ; code de sortie 0 même avec des erreurs de page ; imports en double.
- X-7 `release.yml` : version `0.1.0` écrite en dur ; les essais tournent avant chaque build
  (durée doublée) ; `deploy/README.md` copié deux fois dans l'archive Linux.
- X-8 `cargo audit` absent de la CI. 10 dépendances non maintenues et 1 « unsound » (`glib`),
  presque toutes amenées par GPUI ; `chrono`, utilisée directement, est signalée non maintenue.
- X-9 Service Linux : tourne en root quand le dossier appartient à root (pas de compte dédié).

## 6. Documentation

- D-1 `CLAUDE.md` : « D-01 à D-60 » (réel : D-66) ; « fusion par PR » (consigne : envoi direct) ;
  « seul `unsafe` » dans `platform/` (il y a aussi `biometric/`) ; une ligne de plus de 200
  caractères dans la carte du code.
- D-2 `docs/ETAT.md` : quatre sections « Prochaines étapes » empilées, dont trois périmées.
- D-3 Décisions non marquées remplacées : D-48 (accent, par D-51), D-32 (« VPN seulement, dossier
  lisible par le seul service », contredit par D-63), D-50 (code de configuration, jamais fait),
  D-26 / D-30 (mode dossier partagé pour une équipe sans serveur, journal par appareil : non
  fait, non abandonné), D-19 (export, voir L-4), D-12 (GitHub / SFTP : fait par D-65).
- D-4 `deploy/README.md` : cite « Add an organization server », propose `{tenantid}` (S-8).
- D-5 `README.md` du dépôt : ne mentionne ni le serveur ni Linux.

## 7. Ce qui n'a pas pu être vérifié ici

Windows Hello, Touch ID, DPAPI, trousseau macOS ; service Windows et launchd ; SMTP, passerelle
SMS, Google, Microsoft réels ; GitHub réel (essai contre une imitation locale) ; Docker ; build et
essais macOS ; menus sans option de capture ; lecteur d'écran et navigation au clavier complète ;
largeurs autres que 1440 px (la charte demande ~1700 et ~2400).

## 8. Ordre de correction proposé

1. S-1 (faille de rôle), puis S-2 / S-3 (ce qu'un Admin peut faire seul) : à décider ensemble,
   c'est la même question « que peut l'Admin technique ».
2. L-1, L-2, L-4 : droits réellement appliqués dans l'appli.
3. L-3 : brancher les six réglages ou les retirer de la page.
4. U-1, U-2 : confirmations, Échap, clic ailleurs (un seul composant de dialogue règle aussi
   U-11 et U-12).
5. S-4 à S-8, L-5 à L-8.
6. Noms (U-4), maquette 23, docs, scripts, CI.
