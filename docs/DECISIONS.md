# Cleis — décisions

Gestionnaire de mots de passe interne, usage personnel et d'équipe. Décisions prises le 24/11/2026.

- **D-01 Développement interne** (assouplie par D-27). Pas de produit du marché. Chiffrement par les fonctions de la plateforme
  (navigateur, OS, Node.js), aucune bibliothèque de chiffrement tierce. Services externes acceptés : SMS,
  notifications mobiles, connexion Google / Microsoft, e-mail.
- **D-02 Ergonomie d'abord.** La sécurité avancée est livrée par étapes ou derrière des réglages
  activables. Pas d'audit externe, outil non commercial.
- **D-03 Personnel et équipe.** Un coffre personnel par utilisateur et des espaces partagés par
  organisation. Mode local sans serveur pour un usage personnel ou sans hébergeur.
- **D-04 Deux modes de chiffrement par espace.** Simple (le serveur détient les clés, l'admin restaure)
  par défaut ; bout en bout au choix. Rechiffrement d'un espace d'un mode à l'autre, par l'admin, avec
  sauvegarde préalable. Copie de secours de la clé activable.
- **D-05 Plateformes.** Windows et macOS, en Rust, interface « nulle ou très limitée » (à préciser, voir
  En suspens).
- **D-06 Hébergement** (remplacée par D-20). OVH hébergement web Pro + e-mail OVH. L'offre Pro n'a pas de Node.js : le
  serveur sera en PHP, ou il faudra une offre Cloud Web ou un VPS (en suspens).
- **D-07 Cloudflare** (caduque avec D-26 : serveur joint par VPN, non exposé). Sous-domaine du serveur en DNS seul (sans proxy) : en mode simple, le proxy
  Cloudflare déchiffrerait le trafic et verrait les mots de passe. Le reste du domaine garde le proxy.
  Choix délégué par l'utilisateur.
- **D-08 Vérification en deux étapes.** À l'ouverture de l'application : facultative, activée et
  choisie par l'utilisateur (application, e-mail, SMS, clé). Connexion au serveur : politique de l'admin
  (obligatoire ou non, méthodes autorisées, mémorisation de l'appareil).
- **D-09 Extension de navigateur facultative.** Sans elle : copier-coller ou saisie manuelle. Mode
  d'installation en suspens.
- **D-10 Codes 2FA des comptes partagés** stockés dans le coffre (type de colonne « Code 2FA »).
- **D-11 Colonnes personnalisables.** Ajouter, masquer, réordonner ; types : texte, texte secret,
  nombre, prix, date, liste de choix, code 2FA, calcul, adresse web. Coût annuel calculé.
- **D-12 Sauvegardes chiffrées.** Dépôt GitHub privé ou SFTP. FTP non chiffré exclu. Clé de sauvegarde
  à conserver hors de l'application. Vérification par restauration d'essai.
- **D-13 Import CSV** (sortie de Notion) avec correspondance de colonnes ; export chiffré.
- **D-14 Outils admin.** Journal d'audit, rapport de santé ; historique des versions de chaque élément.
- **D-15 Réglages par défaut.** Effacement du presse-papiers : choix de l'utilisateur. Verrouillage après
  inactivité et biométrie : désactivés, l'admin décide. Générateur de mots de passe discret, jamais imposé.
- **D-16 Hors périmètre.** Partage temporaire avec une personne extérieure ; stagiaire sans accès au mot
  de passe (secondaire : le rôle « Stagiaire » existe, mais la protection n'est pas réelle).
- **D-17 Maquettes avant code**, selon la charte de design du projet (`claude/charte-design-apps.md`).

- **D-18 Langue.** Maquettes et développement en anglais ; versions publiées en anglais et en français
  (fichiers de traduction). Thème sombre / clair / système dans Settings › Preferences (24/11/2026).
- **D-19 Export de l'organisation** autorisé personne par personne par l'admin (colonne « Export » de
  Members) ; les administrateurs peuvent toujours exporter ; chaque export est inscrit au journal.
- **D-20 Stockage au choix, même interface.** Ce poste seul ; serveur de l'organisation (adresse saisie au
  premier lancement, ici le serveur de l'entreprise) ; stockage partagé sans serveur (dossier réseau,
  SFTP, dépôt GitHub privé). Sans serveur : bout en bout uniquement, ni rôles ni journal garantis.
- **D-21 Chiffrement toujours sur le poste** (proposé, à valider) : chaque élément chiffré par la clé de
  son espace ; la clé d'espace est remise chiffrée à chaque membre ; en mode Standard le serveur en garde
  en plus une copie de secours (restauration par l'admin). Le lieu de stockage ne voit que des données
  chiffrées ; HTTPS par-dessus. Conséquence sur D-07 à revoir : le proxy Cloudflare ne verrait plus que des
  données chiffrées.
- **D-22 Application Rust complète** (Windows, macOS). Chiffrement par les API du système : Windows CNG ;
  macOS CryptoKit (Swift seulement pour AES-GCM, petit module Swift d'Apple à intégrer). Interface : choix
  en suspens (fenêtres natives ou vue web du système).
- **D-23 Extension.** Le registre « ExtensionInstallAllowlist » ne force pas l'installation d'une extension
  hors store. Voies retenues à confirmer : Firefox par policies.json (extension signée par Mozilla) ;
  Chrome / Edge / Brave en mode développeur, chargée une fois par poste, guidée par l'app ; Brave peut aussi
  être lancé avec --load-extension.
- **D-24 Codes 2FA.** Codes d'application d'authentification (TOTP) calculés par l'app à partir du secret
  du QR code. Codes reçus par e-mail ou SMS : non calculables ; lecture d'une boîte e-mail en option
  future.

- **D-25 Premier lancement à deux choix** : « This computer only » (coffre personnel) et « Add an
  organization server » ; un seul champ accepte une adresse de serveur, de dossier réseau, SFTP ou GitHub,
  le type est détecté (24/11/2026, demande de l'utilisateur).
- **D-26 Serveur Sésame sur le serveur de l'entreprise**, joint par le VPN existant. Le mode dossier
  partagé reste pour l'usage sans serveur, avec ses limites : droits de lecture par espace assurés par le
  chiffrement ; droits d'écriture, journal fiable et révocation non garantis.
- **D-27 Bibliothèques reconnues autorisées** (demande du 24/11/2026) : aucune primitive de chiffrement
  écrite à la main ; bibliothèques auditées (RustCrypto, curve25519-dalek, rustls), copiées dans le dépôt
  si utile, avec leurs mentions de licence conservées (MIT / Apache-2.0 l'exigent).
- **D-28 Interface à rendu natif** (sans vue web). Moteur : GPUI (D-33).
- **D-29 Safari** : extension emballée dans une app macOS ; diffusion hors App Store par signature
  Developer ID + notarisation (Apple Developer Program, 99 USD / an : dépense à valider).

- **D-30 Journal écrit par l'app**, interrupteur dans *Security policy* (Record activity, Detail : Admin
  actions / Changes / Everything, Keep, Integrity + « Verify now ») — demande du 24/11/2026.
  Avec serveur : le serveur écrit le journal. En dossier partagé : chaque appareil écrit son propre fichier
  (pas de conflit d'écriture), ajout seul ; chaque événement est signé (Ed25519, clé de l'appareil) et lié
  au précédent (SHA-256) ; chaque appareil recopie dans son journal les têtes des journaux des autres
  (témoins), ce qui rend visibles une troncature ou une suppression. Le détail est chiffré pour les admins.
  Limites : falsification détectée, pas empêchée ; une lecture du fichier hors de l'app n'est pas journalisée.
- **D-31 Remplissage sans extension** (proposé) : *Quick Access* au raccourci global (Ctrl+Shift+Space)
  pré-filtré par le contexte, et « Fill best match » (Ctrl+Alt+A). Méthode « Automatic » : écriture dans le
  champ par l'interface d'accessibilité du système, frappe de touches sinon ; « Type » et « Paste » au choix.
  Contexte : programme au premier plan (.exe / bundle id), titre de fenêtre, adresse de l'onglet lue par
  accessibilité (interrupteur, coût de performance dans Chrome/Edge). Applis liées apprises au premier
  remplissage. macOS : permission Accessibilité. Plus tard : fournisseur d'identifiants macOS (signature à
  vérifier) ; plugin passkeys Windows 11 (passkeys seulement). Écrans 16, 17, 19.
- **D-32 Serveur à notre échelle** (proposé) : un binaire Rust unique, service système sur le serveur de
  l'entreprise, écoute sur l'interface VPN seulement, TLS avec empreinte épinglée par le code d'invitation,
  SQLite (WAL) dans un dossier lisible par le seul compte du service ; le serveur ne voit que du chiffré et
  applique droits d'écriture, suppression, révocation, journal, versions, sauvegardes. Le « dossier chiffré
  verrouillé par l'app » demandé revient à ce service. Mode dossier conservé : chaque écriture est signée par
  son auteur ; une écriture d'un non-autorisé est ignorée et signalée (la suppression d'un fichier reste
  possible, détectée et réparable par sauvegarde).
- **D-33 Interface : GPUI** (décision de l'utilisateur, 24/11/2026). Proposé : gpui-component / gpui-kit
  (Longbridge, Apache-2.0) avec la révision GPUI qu'ils fixent, copiée dans le dépôt avec attribution ; une
  seule distribution GPUI dans tout le projet. Accessibilité par AccessKit (intégrée à GPUI). Icône de zone
  de notification et raccourcis globaux : tray-icon et global-hotkey (tauri-apps, MIT / Apache-2.0).
- **D-34 Safari, plan d'essai** : 1) app signée ad hoc par extension.js (sans adhésion) ; 2) non signée.
  La doc d'extension.js indique que les deux exigent « Allow unsigned extensions », que Safari désactive à
  chaque fermeture ; essai à faire sur un Mac (Xcode complet, Node 22.12+). 3) Developer ID (99 USD / an,
  dépense à valider) si le réarmement à chaque lancement est jugé inacceptable.
- **D-35 Nom proposé : Cléis** (affichage), « cleis » pour les identifiants (paquet, bundle id, dossier).
  En attente de confirmation ; recherche de marque INPI / EUIPO à faire avant toute diffusion externe.
- **D-36 « Browser extension » fusionné dans « Autofill »** (Paramètres › My account) : raccourcis,
  méthode, détection, applis liées, extensions par navigateur, permissions.
- **D-37 Nom : Cleis** (sans accent, confirmé le 24/11/2026) ; remplace D-35. Identifiants : « cleis ».
- **D-38 Réglages** (24/11/2026) : raccourcis globaux et remplissage activables un par un (interrupteurs
  dans *Autofill*, plus un interrupteur général) ; usage personnel sans journal (le journal n'existe que
  pour une organisation) ; extension web mise en pause (sections grisées).
- **D-39 Onglet Contacts** (demande du 24/11/2026), activable dans *Preferences › Tabs*. Même système de
  colonnes que le coffre (ajout, suppression, colonnes personnalisées), dossiers personnels et
  d'organisation, étiquettes, favoris, anniversaires, champs multiples typés (Work, Mobile…), éléments du
  coffre liés à un contact, *Merge & fix* (doublons par e-mail, téléphone, nom proche), import / export
  vCard 4.0 (RFC 6350) et CSV. Chiffré comme les mots de passe. Écrans 20 et 21.
- **D-40 Onglets masquables** dans *Preferences › Tabs* : Contacts, Health, Activity (24/11/2026).
  Masquer Activity ne coupe pas l'enregistrement du journal (réglé dans *Security policy*).
- **D-41 Dépôt et auteur** (24/11/2026) : dépôt privé `matv-studio/cleis`. Les commits sont signés
  « Claude <noreply@anthropic.com> » ; aucun nom ni adresse de compte personnel n'apparaît dans l'historique
  (demande de l'utilisateur, remplace la règle générale du projet pour cette app).
- **D-42 Architecture du code** : espace de travail Rust en crates — `cleis-crypto` (formats et
  combinaisons, aucune primitive maison), `cleis-core` (modèle sans E/S : coffre, contacts, colonnes,
  formules, TOTP, imports CSV/vCard, santé, doublons, journal, réglages), `cleis-store` (fichier SQLite où
  tout le sensible est scellé), puis `cleis-app` (GPUI) et `cleis-server`. Vecteurs de test officiels
  (NIST, RFC 9106, 7748, 8032, 6238, 4226) dans les tests unitaires.
- **D-43 CI** : formatage, lint et tests sur Linux et Windows à chaque envoi ; macOS lancé à la demande
  (les minutes macOS comptent 10 fois dans le quota GitHub des dépôts privés : à valider avant de
  l'automatiser).
- **D-44 GPUI par gpui-kit 0.11.0, version figée** (24/11/2026) : la dépendance unique `gpui-kit = "=0.11.0"`
  apporte l'instantané GPUI `gpui-pre 0.3.14` (avec AccessKit) et gpui-component ; `Cargo.lock` fige tout
  l'arbre. La copie du code dans le dépôt (D-33) n'est plus nécessaire tant que crates.io le fournit ; elle
  reste possible si une version disparaît ou casse. Composants de la charte refaits en primitives GPUI
  (`ui.rs`) ; seuls les champs de saisie viennent de gpui-component, recolorés par les jetons.
- **D-45 Création du mot de passe maître** au premier lancement : étape ajoutée sous les deux choix de
  l'écran 03 (deux champs, 10 caractères minimum). Pas de maquette dédiée : à valider sur capture.
- **D-46 Essais de bout en bout** dans `crates/cleis-app/tests/e2e.rs` avec le banc d'essai de gpui-kit
  (fenêtre sans écran, vrais clics et frappes) ; les éléments visés portent `.test_support()`.
- **D-47 Icône des préférences** (25/11/2026) : l'ancienne icône ressemblait à un soleil ; remplacée par le
  rouage de Lucide (ISC, attribution dans `icons.js` et `THIRD_PARTY.md`), ramené à la grille 16 px.
- **D-48 Apparence par défaut** (25/11/2026) : thème sombre, accent `#a57118` (« Bronze », encre noire : meilleur contraste, 5,0:1 ; remplace `#84580b`, changement du
  25/11/2026),
  étendue « Étendue » (onglets, choix, lignes actives en plus de l'action principale, des interrupteurs, de
  la sélection, du focus et du contour du plateau). Réglables dans *Preferences*.
- **D-49 Maquettes Contacts validées** (écrans 20, 21), 25/11/2026.
- **D-50 Connexion GitHub / Microsoft / Google et nouvel appareil** (proposé, 25/11/2026, écrans 03, 22, 23).
  L'identification (qui je suis, où est mon coffre) reste séparée du déchiffrement (mot de passe maître),
  comme chez Bitwarden et 1Password. Se connecter à GitHub, Microsoft ou Google donne accès à
  l'emplacement du coffre chiffré (dépôt privé, dossier d'application OneDrive `Files.ReadWrite.AppFolder`,
  dossier d'application Google Drive `drive.appdata`, non sensible) ; le mot de passe maître reste demandé.
  Sans ces comptes : SFTP saisi à la main, ou « code de configuration » (QR + texte : emplacement et accès)
  affiché par un autre appareil ou imprimé dans un kit d'urgence. La sauvegarde devient une
  synchronisation (fusion par enregistrement, numéros de révision déjà présents dans le fichier).
  Suspens : enregistrement des applications OAuth (GitHub, Microsoft Entra, Google Cloud) au nom de
  l'entreprise ; ouverture sans mot de passe (appareil de confiance) reportée au serveur d'organisation.

- **D-51 Accent « Ochre »** (25/11/2026, remplace l'accent de D-48 ; le reste de D-48 tient) : piste C choisie
  par l'utilisateur, ocre du dossier classique `#e0a93b` (encre noire). Les teintes pâles tombent sous
  3:1 sur le thème clair (WCAG 1.4.11 : focus, états actifs) ; l'appli ajuste donc la luminosité de tout
  accent pour garder 3,6:1 sur la page (`fit_contrast`) : `#e0a93b` reste tel quel en sombre et devient
  ≈ `#a17419` en clair. Pastille « Amber » remplacée par « Teal » (trop proche de l'ocre).
- **D-52 Code `unsafe`** (25/11/2026) : `unsafe_code = "deny"` dans l'espace de travail (au lieu de
  `forbid`), et `#![forbid(unsafe_code)]` à la racine de chaque crate sauf `cleis-autofill`, dont seuls les
  modules `platform/windows.rs` et `platform/macos.rs` l'autorisent (appels Win32 et AppKit, chacun
  commenté). Le reste passe par des bibliothèques : `global-hotkey`, `enigo`, `arboard`, `uiautomation`.
- **D-53 Remplissage, règles retenues** (25/11/2026, applique D-31 ; écrans 16, 17) :
  - Ordre des propositions : appli liée, puis site enregistré (adresse de l'onglet), puis nom dans le titre.
    Dans un navigateur dont l'adresse est connue, le titre seul ne compte pas (une page peut afficher
    n'importe quelle marque). « Fill best match » ne remplit seul qu'un unique lien ou site ; sinon
    Quick Access s'ouvre (et toujours pour un simple nom si « Ask before filling » est actif).
  - Séquence : identifiant, Tab, mot de passe, Entrée si demandé. Si le champ suivant n'est pas un mot de
    passe (connexion en deux pages), Cleis s'arrête : la suite (mot de passe, puis code 2FA) attend le
    prochain « Fill best match » sur la même fenêtre, 2 minutes au plus. Focus déjà dans le mot de passe :
    l'identifiant n'est écrit que si le champ précédent est vide.
  - Avant chaque étape, la fenêtre au premier plan doit être celle demandée, sinon arrêt : un secret n'est
    jamais tapé ailleurs. Raccourcis relâchés avant de taper (Windows).
  - « Automatic » : écriture par UI Automation seulement dans les champs natifs (Win32, WinForms, WPF,
    XAML) ; pages web et contrôles maison : frappe (un changement de valeur par UIA n'y est pas toujours
    vu par la page). « Paste » vide le presse-papiers après 0,4 s.
  - Liens appris : le premier élément choisi dans Quick Access pour une appli (hors navigateurs) ; un
    second choix ne crée pas de lien. Retrait dans *Settings › Autofill › Linked apps*.
  - Raccourcis réglables (bouton « Change », enregistrement de la combinaison ; une touche seule ou
    Shift seul refusés) ; un raccourci pris par une autre appli est signalé « Unavailable ».
  - Limites de cette version : macOS sans lecture du champ ni de l'adresse (auto-type classique, non
    essayé faute de Mac) ; pas d'icône de zone de notification : Cleis doit rester ouvert ; Linux sans
    détection (développement seulement).
- **D-54 Colonnes et import** (25/11/2026, écrans 02 et 14) :
  - Menu *Columns* : clic = afficher / masquer, glisser = réordonner (déposer sur une colonne de l'autre
    groupe l'affiche ou la masque aussi), corbeille au survol pour une colonne personnalisée. Les valeurs
    des colonnes personnalisées se saisissent dans l'inspecteur, lues selon le type (date, nombre, prix,
    liste…) ; une valeur refusée garde le champ en rouge et n'est pas écrite. Formules : calculées, non
    saisissables.
  - Import : étapes 1 (Fichier) et 3 (Vérification) ajoutées aux maquettes (14a, 14c). Une colonne Cleis
    ne reçoit qu'une colonne du fichier (la précédente passe à *Skip*). Un élément déjà présent (même
    espace, même nom et identifiant, sans tenir compte des accents ni de la casse) est ignoré par défaut.
  - Page *Import & export* sous *My account* tant que le serveur n'existe pas (dans les maquettes elle
    est dans le groupe de l'organisation). L'export chiffré reste grisé jusqu'à *Sync & backups*.
  - Quick Access sous Windows : plus d'appui simulé sur Alt (GPUI et la reprise de Cleis) ; il arrivait
    à l'appli au premier plan, qui affichait ses touches d'accès et gardait le clavier (essai Bloc-notes
    du 25/11/2026). Reprise par une touche F22 factice, comme Chromium.
- **D-55 Arrière-plan** (25/11/2026, remplace la limite « pas d'icône » de D-53) : icône dans la zone
  de notification (Windows) ou la barre des menus (macOS), menu natif *Open Cleis*, *Lock*, *Quit
  Cleis* ; un clic sur l'icône ouvre la fenêtre (Windows). Fermer la fenêtre la cache : les raccourcis
  restent actifs ; quitter se fait par le menu de l'icône. *Open at login* (activé par défaut, D-38)
  écrit `HKCU\…\Run` (Windows) ou un LaunchAgent (macOS) avec `--background` (icône seule au
  démarrage) ; rien n'est écrit pour le coffre de démonstration. Garde-fou du remplissage : aucune
  valeur n'est écrite si le focus n'est pas dans un champ de texte ; masque de la touche Alt (0xE8)
  quand un raccourci la contient.
- **D-56 Contacts** (25/11/2026, écrans 20, 21) : édition dans l'inspecteur comme pour le coffre
  (nom en une case : dernier mot = nom de famille ; poste et société sous le nom). Valeurs multiples :
  clic sur le type (Work, Mobile…) pour passer au suivant, « + » ajoute une ligne et y place le
  curseur, une ligne vidée est retirée. Adresse saisie sur une ligne (rue, code postal ville, pays) ;
  anniversaire `Mar 12`, `12/03`, `12 mars 1990`. Étiquettes : « + » (liste des existantes ou nouvelle),
  croix au survol. Onglets Details / Linked (éléments du coffre liés, copie du mot de passe) /
  History. Dossiers de l'organisation créés par « + ». *Merge & fix* : le premier groupe s'ouvre, le
  contact le plus complet est gardé (clic sur l'autre carte pour inverser) ; « Not a duplicate » est
  retenu dans les préférences du coffre. Import vCard depuis la barre latérale (dans le dossier
  affiché) ; *Share vCard* enregistre un fichier .vcf. Suppression après confirmation, définitive.
  Reste : menu *Columns* et filtres pour les contacts, import CSV de contacts.
- **D-57 Sécurité du compte** (25/11/2026, écrans 05, 06) : code de vérification à l'ouverture par
  appli d'authentification (clé de 160 bits, QR + clé à recopier, activation après un code juste,
  8 codes de secours à usage unique) ; e-mail, SMS et clé de sécurité attendent le serveur. Le code est
  vérifié une fois le coffre ouvert : il protège l'usage de Cleis sur cet ordinateur, pas le fichier
  copié (protégé par le mot de passe maître). Verrouillage après inactivité pour le coffre personnel
  (Never, 1, 5, 15, 30 min, 1 h ; résout « En suspens » 5). Effacement du presse-papiers réglable
  (10 s à 2 min). Changement du mot de passe maître : mot de passe actuel vérifié, 10 caractères au
  moins, mêmes réglages Argon2id. Biométrie : grisée (prévue).

## En suspens
1. Serveur de l'entreprise : système (Windows ou Linux).
2. Safari : résultat de l'essai (D-34), puis adhésion Apple ou non.
3. Validation de l'étape « mot de passe maître » du premier lancement (D-45).
4. Validation de la restauration et de la synchronisation (D-50, écrans 22, 23).
5. Essai du remplissage sur Windows (pilotage d'écran) puis sur un Mac.
