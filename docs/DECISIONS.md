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
- **D-12 Sauvegardes chiffrées.** *(Fait par D-65.)* Dépôt GitHub privé ou SFTP. FTP non chiffré exclu. Clé de sauvegarde
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
- **D-19 Export de l'organisation** *(Appliqué à l'export vCard des contacts partagés, journalisé ; aucun export des éléments d'organisation n'existe : l'export chiffré ne prend que le coffre personnel.)* autorisé personne par personne par l'admin (colonne « Export » de
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
- **D-26 Serveur Sésame sur le serveur de l'entreprise** *(Mode dossier partagé sans serveur : non fait, sans demande ; le serveur universel D-63 couvre le besoin.)*, joint par le VPN existant. Le mode dossier
  partagé reste pour l'usage sans serveur, avec ses limites : droits de lecture par espace assurés par le
  chiffrement ; droits d'écriture, journal fiable et révocation non garantis.
- **D-27 Bibliothèques reconnues autorisées** (demande du 24/11/2026) : aucune primitive de chiffrement
  écrite à la main ; bibliothèques auditées (RustCrypto, curve25519-dalek, rustls), copiées dans le dépôt
  si utile, avec leurs mentions de licence conservées (MIT / Apache-2.0 l'exigent).
- **D-28 Interface à rendu natif** (sans vue web). Moteur : GPUI (D-33).
- **D-29 Safari** : extension emballée dans une app macOS ; diffusion hors App Store par signature
  Developer ID + notarisation (Apple Developer Program, 99 USD / an : dépense à valider).

- **D-30 Journal écrit par l'app** *(Journal tenu par le serveur depuis D-59 ; le journal par appareil n'est pas fait.)*, interrupteur dans *Security policy* (Record activity, Detail : Admin
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
- **D-32 Serveur à notre échelle** *(En partie remplacé par D-63 : écoute sur toutes les interfaces par défaut, réglable ; dossier restreint au service, Windows compris depuis l'audit S-7.)* (proposé) : un binaire Rust unique, service système sur le serveur de
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
- **D-48 Apparence par défaut** *(Accent remplacé par D-51 ; le reste tient. D-51 prime sur la charte « aucun accent par défaut » pour Cleis : règle propre à l'app, audit U-13.)* (25/11/2026) : thème sombre, accent `#a57118` (« Bronze », encre noire : meilleur contraste, 5,0:1 ; remplace `#84580b`, changement du
  25/11/2026),
  étendue « Étendue » (onglets, choix, lignes actives en plus de l'action principale, des interrupteurs, de
  la sélection, du focus et du contour du plateau). Réglables dans *Preferences*.
- **D-49 Maquettes Contacts validées** (écrans 20, 21), 25/11/2026.
- **D-50 Connexion GitHub / Microsoft / Google et nouvel appareil** *(Code de configuration jamais fait ; GitHub par jeton (D-65), OneDrive / Google Drive par leur appli de bureau ; OAuth en suspens 4.)* (proposé, 25/11/2026, écrans 03, 22, 23).
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
- **D-58 Synchronisation par dossier** (25/11/2026, applique D-50 sans attendre l'enregistrement OAuth) :
  la place de synchronisation est un dossier ; OneDrive et Google Drive passent par leur appli de
  bureau (dossier synchronisé), un partage réseau, Dropbox ou une clé USB par « Folder ». GitHub et
  SFTP restent grisés. Fusion ligne par ligne sans déchiffrer : chaque enregistrement, espace et
  réglage garde la version modifiée en dernier (colonne `changed`, millisecondes, ajoutée par une
  migration) ; un élément effacé définitivement est retenu (`purged`) et ne revient pas ; un mot de
  passe maître changé ailleurs est repris (même clés de compte, scellées autrement). La copie fusionnée
  remplace celle du dossier en une fois (écriture à côté puis renommage). Synchronisation à
  l'ouverture, toutes les 5 minutes et au verrouillage. Réglages de la place propres à l'ordinateur
  (`<coffre>.sync.json`). Sauvegardes datées sur l'ordinateur (une par jour, « Back up now », 10 / 30
  / 100 versions), restauration par fusion. Export chiffré = copie du coffre (mot de passe maître).
  Kit d'urgence : PDF d'une page (emplacements, place pour écrire le mot de passe). « Restore my vault »
  au premier lancement (écran 22). Code de configuration : avec les comptes GitHub / Microsoft / Google.
  Contacts supprimés : effacés définitivement (remplace la suppression douce).

- **D-59 Serveur d'organisation** (25/11/2026, applique D-21, D-26, D-30, D-32 ; écrans 03, 05b, 07–11,
  13, 18, 24, 25) :
  - `cleis-server` : un binaire, un fichier SQLite (WAL, `secure_delete`), HTTPS par rustls avec un
    certificat Ed25519 auto-signé fait à `init` ; commandes `init`, `run --listen ADRESSE_VPN:PORT`,
    `invite`, `rotate --finish`, `status`. Il ne détient aucune clé qui ouvre un élément.
  - Confiance : le code d'invitation porte un jeton à usage unique (10 octets) et les 10 premiers
    octets du SHA-256 du certificat ; l'app n'accepte que ce serveur dès la première connexion puis
    garde l'empreinte entière. Code de 32 caractères (8 groupes) au lieu des 8 de la maquette :
    un code court exigerait un échange PAKE (SPAKE2, CPace) pour ne pas être devinable hors ligne ;
    à reconsidérer si la longueur gêne. Rotation : nouveau certificat annoncé par la connexion
    épinglée, mis en service au redémarrage 7 jours après (ou `rotate --finish`).
  - Connexion : signature Ed25519 de la clé du compte sur un défi du serveur lié au certificat ; code
    d'appli d'authentification si le membre en a un (rejeu refusé, appareil mémorisé N jours selon la
    politique) ; si la politique l'exige et qu'il n'en a pas, session limitée à sa mise en place.
    E-mail, SMS, comptes Google / Microsoft, clés de sécurité : grisés (service extérieur ou dépense).
  - Clés : chaque clé d'espace est enveloppée pour chaque membre ; un membre ajouté attend qu'une app
    qui tient la clé l'enveloppe pour lui (membre de l'espace, ou admin par la clé d'organisation).
    Clé d'organisation (X25519) faite par le premier admin, enveloppée pour chaque admin ; espaces
    Standard = copie enveloppée pour elle. Clé de secours facultative, hors ligne (fichier texte), pour
    tous les espaces ; « Use » redonne leurs espaces aux membres en attente.
  - Droits vérifiés par le serveur : Edit / Delete (lignes envoyées), Manage, Audit ; See est appliqué
    par l'app (le poste tient la clé). Rôles par défaut Admin, Member, Read only, Intern ; rôles créés
    par les admins. Le dernier admin ne peut être rétrogradé ni retiré.
  - Synchronisation : lignes scellées, la plus récente gagne (`changed`), versions gardées (20 par
    élément), suppressions définitives propagées ; espaces d'organisation hors des copies personnelles
    et des sauvegardes personnelles. Retrait d'un membre ou d'un espace : l'espace est marqué
    « Re-encrypt advised » ; le rechiffrement (nouvelle clé, tout rescellé, génération +1) se fait
    depuis Encryption, avec changement de mode Standard ↔ End-to-end.
  - Journal écrit et signé par le serveur, chaîné, rétention par ancre (le reste se vérifie) ; vues,
    copies, remplissages, exports signalés par l'app avec le nom de l'élément scellé pour la clé
    d'organisation (lisible des admins seuls). Export refusé par le serveur sans l'autorisation
    « Export ».
  - Sauvegardes de l'organisation : copie du fichier du serveur scellée avec une clé de sauvegarde
    (gardée dans le coffre de l'admin, téléchargeable), vérification, restauration (même organisation,
    même certificat ; sinon par la ligne de commande).
  - Politique appliquée par l'app : délai de verrouillage plafonné, accès hors ligne (espaces masqués
    au-delà), seuils du rapport de santé, méthodes de remplissage. Heures affichées à l'heure locale.
- **D-60 Super Admin, sans clé de sécurité** (25/11/2026, demande de l'utilisateur ; écrans 07, 07e,
  26–28) :
  - Rôle « Super Admin » donné au créateur de l'organisation. Il peut être transféré à un autre membre
    (menu ⋯ du membre › « Transfer Super Admin » : l'autre devient Super Admin, soi-même Admin) ou
    ajouté à un autre (« Make Super Admin ») ; il en reste toujours au moins un.
  - Ce qui lui est réservé (droit « Admins, roles, server keys ») : donner ou retirer les rôles Admin
    et Super Admin, inviter ou retirer un admin, modifier les rôles, clé de secours, rotation de la clé
    du serveur, restauration d'une sauvegarde. Admin garde tout le reste (membres, espaces, politique,
    chiffrement des espaces, journal, sauvegardes). Proposé, à ajuster à l'audit si besoin.
  - Les deux rôles tiennent la clé d'organisation et exportent toujours. Un serveur antérieur reçoit
    le rôle à l'ouverture (premier admin promu).
  - Clés de sécurité retirées partout (déverrouillage, code à l'ouverture, méthodes de la politique ;
    remplace la mention de D-08).
  - Pages ajoutées : Profile (nom, e-mail, empreinte de la clé du compte), Devices (appareils
    connectés ou mémorisés, déconnexion), Spaces (renommer, supprimer un espace et son contenu pour
    tous, avec confirmation).
  - Captures : thème sombre seulement désormais (`THEMES="dark light"` pour les deux).
- **D-61 Espaces réservés** (25/11/2026, demande de l'utilisateur : comptabilité ou direction hors
  de portée de l'administrateur technique) :
  - Case « Reserved » d'un espace (Spaces, dialogue New space), réservée au Super Admin. Un espace
    réservé est forcément End-to-end (sans copie pour la clé d'organisation) ; réserver un espace
    Standard ouvre d'abord son rechiffrement en End-to-end.
  - Le serveur refuse à tout autre qu'un Super Admin : changer ses membres (y compris s'y ajouter),
    l'inviter, le renommer, le supprimer, le rechiffrer, le libérer. Les admins qui n'en sont pas
    membres ne le voient pas (ni dans l'état, ni dans les listes de membres, ni dans le journal).
  - Chaque accès garde qui l'a donné ; une app ne remet la clé d'un espace réservé qu'à un accès
    donné par un Super Admin (défense en profondeur, si le serveur était trompé).
  - Clé Super Admin (X25519) faite par le premier Super Admin, enveloppée pour chacun, retirée à
    celui qui perd le rôle : le nom des éléments d'un espace réservé, au journal, est scellé pour elle
    (lisible des Super Admins seuls).
  - Corrige le trou relevé le 25/11 : un Admin pouvait s'ajouter à un espace End-to-end et recevoir
    sa clé de l'app d'un membre.
- **D-62 Dossiers personnels** (25/11/2026, demande de l'utilisateur) : un dossier est un espace
  du coffre personnel (sa propre clé). « + » du groupe Personal, « ⋯ » au survol : renommer,
  supprimer (My vault ne se supprime pas ; le contenu, corbeille comprise, revient dans My vault,
  rescellé avec sa clé). Déplacement d'un élément par le menu de l'inspecteur (« Move to … »).
  « All items » regroupe tous les dossiers. Un dossier supprimé est noté effacé : une
  synchronisation ne le ramène pas. Maquettes 01d, 01e.
- **D-63 Serveur universel, organisation créée depuis l'appli** (25/11/2026, demande de
  l'utilisateur : ne pas attendre un système ou un type de serveur, ne rien brider) :
  - `cleis-server` tourne partout : Linux (systemd), Windows (service SCM, `windows-service`),
    macOS (launchd), Docker (`deploy/`). `install` / `uninstall` posent le service ; `--print`
    affiche le fichier sans rien écrire. Dossier par défaut propre à chaque système
    (`CLEIS_DATA` sinon). Sous Linux, un dossier gardé dans un répertoire personnel garde son
    propriétaire (`User=` du service).
  - `cleis-server.toml` dans le dossier : adresses d'écoute (IPv4 et IPv6 par défaut), sauvegardes
    automatiques (toutes les 24 h, 30 gardées), et sections prêtes pour l'e-mail (SMTP), les SMS
    (passerelle HTTP) et la connexion Google / Microsoft (OpenID Connect). Commandes ajoutées :
    `invite`, `backup`, `restore`, `status`, `rotate --finish` ; `/v1/health` pour la supervision.
  - Settings › Organization › Create an organization (maquette 25b) : « This computer » lance le
    `cleis-server` livré à côté de Cleis, dans le dossier de données de l'utilisateur, sur le premier
    port libre à partir de 8443, et rejoint en premier admin (Super Admin) ; « Another machine »
    donne les commandes à copier. Le serveur tourne tant que Cleis tourne (la zone de notification
    le garde ouvert), s'arrête avec lui, redémarre à l'ouverture du coffre ; « Install » le pose en
    service système (droits administrateur demandés).
  - Page Server d'une organisation hébergée ici (maquette 24b) : état, adresses à donner aux
    membres (copie), Install.
  - Versions : `cleis-server` livré dans les archives Windows et macOS, archive Linux à part.

- **D-64 Deuxième étape de connexion par e-mail, SMS, Google, Microsoft** (25/11/2026, demande
  de l'utilisateur : implémenter ce qui dépend d'un service extérieur, prêt dès sa configuration) :
  - Le serveur dit ce qu'il sait faire (`[smtp]`, `[sms]`, `[[oidc]]` de `cleis-server.toml`) ;
    la politique de sécurité n'offre que ces méthodes (case grisée sinon, info-bulle : quelle section
    ajouter). Un membre en choisit une (Profile › Second step, ou à la demande de la politique).
  - E-mail (lettre, SMTP) et SMS (n'importe quelle passerelle HTTPS, gabarit `{to}` `{code}`
    `{text}`) : code à 6 chiffres gardé haché 10 min, 5 essais, un envoi par minute au plus.
  - Google / Microsoft : OpenID Connect, code d'autorisation avec PKCE et redirection locale
    (`http://127.0.0.1:<port>`, RFC 8252) ; le navigateur affiche la page du fournisseur. Le serveur
    vérifie le jeton d'identité (signature RS256 par les clés publiées, émetteur, audience,
    expiration, domaine), le lie au membre (émetteur + sujet) et ne l'accepte qu'une fois.
    RS256 passe par `cleis-crypto` (`ring`), essayé sur le vecteur de la RFC 7515 A.2.
  - Aucun compte ni enregistrement n'est fait par Cleis : l'organisation crée son client OAuth
    (Google « Desktop app », Microsoft Entra) et sa passerelle SMS ; guide dans `deploy/README.md`.

- **D-65 Places GitHub et SFTP** (25/11/2026, même demande) : remplace l'attente d'un
  enregistrement OAuth (D-50, D-58) pour ces deux places.
  - GitHub : un fichier d'un dépôt privé, par l'API « contents » et un jeton d'accès personnel
    (fine-grained, ce dépôt seulement, Contents en lecture-écriture) ; aucune appli à enregistrer.
    Adresse complète acceptée (GitHub Enterprise : API sous `/api/v3`). L'envoi donne le sha lu :
    si un autre appareil a envoyé entre-temps, relecture, fusion, nouvel envoi (3 essais).
  - SFTP : `russh` + `russh-sftp` (Rust pur, `ring`), mot de passe ou clé privée OpenSSH. Clé du
    serveur montrée à la première connexion (« Connect to trust »), vérifiée ensuite : une autre
    clé arrête la synchronisation. Écriture à côté puis renommage.
  - Le fichier est lu dans une copie locale (`<coffre>.remote`), fusionné comme une place dossier
    (lignes scellées), renvoyé s'il a changé. Le secret de la place est scellé dans le coffre.
  - Premier lancement : restauration depuis GitHub ou SFTP (mêmes dialogues). Sauvegardes de
    l'organisation : copie scellée de chaque sauvegarde dans un dossier GitHub ou SFTP.
  - Maquettes 23b, 23c. Essais : `sync_github_place` (faux GitHub local), `openssh_round_trip`
    (vrai serveur OpenSSH quand il est installé).

- **D-66 Déverrouillage par Windows Hello / Touch ID** *(Retiré par D-71.)* (25/11/2026, même demande) :
  - Sur cet appareil seulement (Settings › Security › Unlock with …) : une clé aléatoire scelle
    les secrets du compte dans `<coffre>.bio` ; le système la garde pour l'utilisateur (Windows :
    DPAPI ; macOS : trousseau de session) et Cleis ne la lit qu'après sa vérification (Windows :
    `UserConsentVerifier`, au-dessus de la fenêtre sous Windows 11 ; macOS : LocalAuthentication,
    Touch ID ou mot de passe du Mac). Linux : pas de méthode, l'option est grisée.
  - Le mot de passe maître reste la voie d'entrée (autres appareils, après désactivation) ; le
    changer ne casse pas la clé (mêmes secrets de compte).
  - Organisation : la politique « Biometrics » (désactivée par défaut) l'autorise ; désactivée,
    la clé de l'appareil est retirée à la synchronisation suivante.
  - Code système dans `cleis-autofill::biometric` (seul crate où `unsafe` est permis, D-52),
    vérifié par clippy pour Windows et macOS ; essai `biometric_unlock` avec une doublure
    (`test-support`). Reste à essayer sur un vrai Windows Hello et un vrai Touch ID.

- **D-67 Droits délégués à l'Admin** (25/11/2026, après l'audit complet) :
  - Quatre droits « délégables » : journal et rapport de santé (`Audit`), sauvegardes du serveur
    (`Backups`), clé de récupération et clés du serveur (`Keys`), règles de connexion et du journal
    (`Rules`). Seul un Super Admin les coche, et seulement dans la colonne Admin ; par défaut
    l'Admin n'a que le journal. Les autres rôles n'en reçoivent aucun.
  - Pourquoi : le Super Admin (comptabilité ou direction) décide ce qu'il confie ; la sauvegarde du
    serveur contient la clé du serveur et les clés des authentificateurs des membres.
  - Le serveur vérifie chaque droit (`backup`, `recovery`, `rotate`, `set_policy` si une règle
    change, `reset_step`) ; l'appli grise et verrouille les sections (« Set by a Super Admin »).
  - Corrige aussi S-1 (une invitation d'admin ne se renouvelle que par un Super Admin), L-1 (un
    changement refusé par le rôle est remis), L-2 (un élément de l'organisation n'en sort pas).
  - Maquettes 07f, 08b.

- **D-68 Menus contextuels, confirmations, Échap** (25/11/2026) :
  - Clic droit sur un élément du coffre, un espace ou un dossier de la barre latérale, un contact,
    un membre (maquettes 33 à 33d) : menu à la position du pointeur, sous-menus « ▸ » au survol,
    actions grisées avec leur raison en info-bulle (droits du rôle, règle L-2). Le menu « ⋯ » d'un
    membre et le clic droit partagent la même liste.
  - Raccourcis de l'élément choisi, quand aucune case n'a le clavier : Ctrl+C identifiant,
    Ctrl+Maj+C mot de passe, Ctrl+Alt+C code 2FA, Ctrl+D dupliquer, Suppr corbeille (⌘ sur Mac) ;
    Suppr sur un contact demande confirmation.
  - Confirmation (bouton rouge, Annuler a le focus ; Entrée et Échap annulent) avant : retirer un
    membre, transférer ou donner le rôle Super Admin, réinitialiser la vérification en deux
    étapes, déconnecter un membre partout, révoquer une invitation, quitter l'organisation,
    changer la clé du serveur, déconnecter la place de synchronisation (audit U-1, maquettes 34).
  - Échap ferme le menu ou le dialogue du dessus ; un clic hors d'un menu contextuel le ferme
    sans atteindre l'élément dessous (audit U-2).
  - Nouvelles routes du serveur : `/v1/members/reset-step` (droit Rules), `/v1/members/sign-out`
    (Manage) ; un admin visé demande le droit Admins.
  - Nom unique « Two-step verification » / « 2-step » pour la deuxième étape du serveur (U-4).

- **D-69 Onglets de coffre, plusieurs organisations par coffre** (25/11/2026, maquettes 32 à 32d) :
  - Preferences › Vaults : « Personal vault in its own tab » (désactivé par défaut : un seul onglet
    Vault, tous les coffres dans sa barre latérale) et le nom du coffre personnel (« My vault »,
    son premier espace renommé). Activé : le coffre personnel à gauche, puis un onglet au nom de
    chaque organisation, dans l'ordre d'adhésion ; chaque onglet a sa barre latérale (Folders,
    ou les espaces de l'organisation).
  - Un coffre peut appartenir à plusieurs organisations : réglage `orgs` (liste ; `org` garde la
    première pour les versions précédentes), un fichier local par organisation
    (`<coffre>.org-<id>.json`, l'ancien `.org.json` lu pour la première).
  - L'appli garde une organisation « montrée » (`Session::org`) et les autres à côté
    (`orgs_idle`) ; `in_org(clé, …)` met l'organisation voulue en place le temps d'une
    synchronisation, d'une réponse du serveur ou d'un rapport d'activité, puis remet la montrée.
    Pourquoi : les ~190 usages de `s.org` restent justes sans tout réécrire.
  - Droits, politique de remplissage, rapport de santé, verrouillage (le plus court), lecture des
    adresses (la plus stricte) : selon l'organisation de l'espace de l'élément. Un élément ne passe
    pas d'une organisation à une autre.
  - Réglages : un groupe par organisation ; « Add an organization » ouvre la page de jonction.
    Activity : l'organisation est le premier filtre quand plusieurs donnent le droit au journal.
  - Health : filtre de coffre en premier (tous, personnel, chaque organisation), puis les espaces
    de ce coffre (maquette 32e, demande du 25/11/2026 à 16 h).

- **D-70 Fin de l'audit complet** (25/11/2026, « pas de version tant que l'audit n'est pas fini ») :
  - Sécurité : plancher Argon2 (OWASP 19 Mio, 2 passes ; un coffre en dessous est relevé après
    déverrouillage), jeton GitHub en HTTPS seulement hors essais, jeton « appareil mémorisé » scellé
    dans le coffre, jetons d'identité récents (`iat`, `max_token_age` réglable, 600 s par défaut) et
    émis pour Cleis (`azp`), 10 codes e-mail / SMS par jour et par membre, actions GitHub épinglées
    par empreinte, restauration refusée si le journal de la copie ne se vérifie pas et membres
    retirés depuis gardés dehors, service Windows lancé depuis `Program Files` et dossier de
    données sans héritage, service Linux sous un compte `cleis-server` quand le dossier est à root.
  - Logique : adresse du serveur modifiable (clé épinglée inchangée), nom saisi à la création d'une
    organisation, groupes de contacts par organisation, invitations par e-mail quand `[smtp]` est
    réglé, Activity masquable, proposition de couper la biométrie après un nouveau mot de passe.
  - Interface : clic ailleurs ferme tout menu (le clic ne passe pas dessous, comme les menus
    système), Ctrl+N et flèches, noms unifiés, gris et voile de la charte, un seul dialogue de 420 px,
    explications en info-bulle (marque « i »), menus de 220 px.
  - Outils : licences dans les archives (`tools/licenses.py`), CI sur `feat/**` et `fix/**`, essais
    sautés interdits sur Linux (`CLEIS_REQUIRE_ALL`), `cargo audit` (échec sur vulnérabilité ; les
    crates non maintenues viennent presque toutes de GPUI, `chrono` aussi), `.dockerignore` et
    contrôle de santé qui joint le serveur, scripts de captures qui échouent quand une vue échoue.
  - Restent à décider (voir « En suspens » 10 à 14).

- **D-71 Choix de fin d'audit** *(S-9 remplacé par D-76 : Windows Hello revient, par une clé du TPM.)* (25/11/2026, réponses de l'utilisateur) :
  - S-9 : le déverrouillage par Windows Hello / Touch ID est retiré (appli, politique, module
    système, fonctions du coffre) : la clé gardée par DPAPI ou le trousseau restait lisible par
    tout programme de la session. Le fichier `<coffre>.bio` d'une version précédente est effacé à
    l'ouverture. Le mot de passe maître (et le code à l'ouverture) restent les seules entrées.
  - S-11 : les sauvegardes automatiques du serveur restent en clair, dans le dossier choisi dans
    `cleis-server.toml`, sans restriction ; le guide dit ce qu'elles contiennent.
  - L-9 : la branche GitHub est élaguée chaque semaine aux dernières versions (voir D-72).
  - U-9 : interface traduite en français (fichiers de traduction EN + FR, langue choisie dans
    Preferences) ; D-18 est remplacée sur ce point.
  - U-17 : les fonctions prévues mais non faites (fuites de mots de passe, extension de
    navigateur) sont retirées de l'interface et des maquettes jusqu'à leur arrivée.

- **D-72 Élagage de la branche GitHub** (25/11/2026, choix L-9) : une fois par semaine et par
  appareil, après une synchronisation réussie, Cleis réécrit la branche de la place GitHub avec
  ses 30 dernières versions (mêmes arbres et messages, sans parent plus ancien) et la remplace
  (`force`). Si un autre appareil a écrit pendant ce temps, rien n'est remplacé : nouvel essai la
  semaine suivante. L'historique plus ancien des autres fichiers de cette branche part aussi :
  une place GitHub vise un dépôt ou une branche qui lui est réservé. Code : `remote::prune`
  (essais `prune_keeps_the_last_versions`, `prune_steps_back_when_the_branch_moved`).

- **D-73 Traduction de l'interface** (25/11/2026, choix U-9) : le code garde l'anglais comme clé ;
  `crates/cleis-app/i18n/fr.json` donne le français (environ 950 entrées). `i18n::tr` (texte
  fixe), `trf!` (gabarit à `{}` ou `{N}`, l'ordre peut changer en français), `tr_owned` (texte
  venu d'une constante), `tr_dynamic` (message fait ailleurs : erreurs du cœur, du coffre, du
  serveur, avertissements d'import ; le gabarit qui colle le mieux gagne). Les composants
  (`chip`, `segmented`, `field`, `menu_item`, info-bulles, boutons de confirmation) traduisent
  eux-mêmes leur libellé. Pluriels : `élément{}` avec le même suffixe « s » qu'en anglais.
  - Langue : Preferences › Language region, gardée dans le coffre et dans `appearance.json` (écran
    de déverrouillage). Au premier lancement, la langue du système (`sys-locale`) ; un coffre
    neuf prend la langue affichée. Un changement s'applique tout de suite : les cases de
    l'inspecteur sont refaites, les placeholders des recherches changent.
  - Données non traduites : noms d'espaces et de colonnes choisis par l'utilisateur ; seuls les
    noms par défaut (« My vault », « My contacts », colonnes et rôles d'origine, types d'adresse
    Work / Mobile…) le sont, à l'affichage. Le journal d'activité garde les clés anglaises
    (`password`, `username`) : chaque membre les lit dans sa langue.
  - Montants : forme `€69.99` conservée en français (le format monétaire régional reste à faire,
    voir « En suspens » 15) ; la période (« / mois ») est traduite et relue par l'analyseur.
  - Essai : `tests/lang.rs` (binaire à part, la langue est un réglage du processus) ; vérification
    que chaque gabarit français garde les champs de l'anglais (`i18n::tests`). Captures :
    `UI_LANG=fr design/tools/appshots.sh`.
  - Le kit d'urgence (PDF) et la ligne de commande du serveur restent en anglais.

- **D-74 Mises à jour** (26/11/2026, demande de l'utilisateur, présentation reprise de Revel) :
  Cleis lit les versions publiées du dépôt public `matv-studio/Cleis` (API GitHub, sans jeton)
  au démarrage puis toutes les six heures. Une version plus récente fait paraître le bouton
  « Version x.y.z » en haut à droite ; Preferences › Updates donne la version installée, l'option
  « Recevoir les préversions » (par défaut : oui, toutes les 0.x en sont) et « Rechercher les
  mises à jour ». Les brouillons ne comptent jamais.
  - Installation (Windows) : par l'installateur, voir D-75 (remplace le remplacement des fichiers
    du zip portable, retiré avant toute version).
  - Ailleurs (macOS, Linux) : le bouton ouvre la page de la version.
  - Limite : la somme `.sha256` vient de la même version GitHub ; elle protège d'un
    téléchargement abîmé, pas d'un compte GitHub compromis. Signature des paquets (clé Ed25519
    dans les secrets du dépôt) : à décider, « En suspens » 16.
  - Jamais dans la démo, les captures ni les essais (`update.enabled` mis par `main.rs`).
    Captures : `--update 0.2.1 [--menu update]`.

- **D-75 Installateur Windows et une seule copie de Cleis** (26/11/2026, demande de l'utilisateur
  après un doublon de la version portable) : à partir de la 0.2.1, Windows reçoit
  `Cleis-x.y.z-<commit>-windows-x64-setup.exe` à la place du zip portable.
  - NSIS (`packaging/windows/cleis.nsi`) : licence zlib, sans dépense ; absent de l'image
    windows-2025 de GitHub, installé par Chocolatey au moment de la construction. Écarté : Inno Setup (absent de l'image windows-2025, licence commerciale
    demandée aux entreprises), MSI/WiX (installation par machine plus lourde, UAC).
  - Installation par utilisateur, sans administrateur : `%LOCALAPPDATA%\Programs\Cleis`, raccourcis
    menu Démarrer et bureau, entrée « Applications installées » (HKCU), désinstalleur qui garde
    le coffre et retire l'ouverture à la connexion. Anglais et français. Icône :
    `packaging/windows/cleis.ico` (tiré du logo, voir `design/logo/README.md` ; le 26/11/2026 :
    logos clair et sombre fournis, version claire pour l'exécutable, l'installateur et la zone de
    notification, la version du thème dans l'appli ; icône de l'exécutable posée par `build.rs`).
  - Une seule copie par utilisateur (`instance.rs`) : verrou `instance.lock` dans le dossier de
    données ; la copie qui le tient écoute sur un port local, noté avec un jeton aléatoire dans
    `instance.port`. Un second lancement fait revenir la fenêtre puis s'arrête (au démarrage de
    session : s'arrête seulement) ; `Cleis --quit` ferme la copie ouverte (le serveur qu'elle a
    lancé s'arrête avec elle). Le verrou part avec le processus : un plantage ne bloque rien.
    La démo et les captures n'y participent pas.
  - Mises à jour (D-74) : Cleis télécharge l'installateur et sa somme, le lance avec `/S /UPDATE` et
    se ferme ; l'installateur attend sa fermeture, installe, relance Cleis. Le remplacement des
    fichiers en place (zip portable) est retiré.

- **D-76 Ouverture de Cleis : mot de passe maître, Windows Hello ou rien ; clé de récupération**
  (26/11/2026, maquettes 35h à 35k validées) :
  - Par ordinateur (Settings › Security › Opening Cleis, et au premier lancement) : le mot de
    passe maître, Windows Hello, ou rien. Pour les deux derniers, une clé de cet ordinateur scelle
    les secrets du compte dans `<coffre>.unlock`, à côté du coffre mais jamais synchronisé.
  - Windows Hello comme KeePassXC : une paire de clés du magasin Windows Hello (dans le TPM s'il y
    en a un) signe un défi aléatoire gardé dans ce fichier ; la signature (RSA PKCS#1 v1.5,
    déterministe), obtenue seulement après l'invite Windows Hello, donne la clé par SHA-256.
    Corrige le défaut S-9 qui avait fait retirer D-66 (D-71) : un programme de la session ne lit
    plus la clé, il devrait passer par l'invite. L'invite, ouverte parfois derrière la fenêtre,
    est remise devant (fenêtre « Credential Dialog Xaml Host »).
  - « Rien » : clé aléatoire scellée par DPAPI pour le compte Windows (tout programme de la session
    peut la lire : l'info-bulle le dit). Cleis s'ouvre seul au lancement. Linux et macOS : pas
    de Windows Hello ; « rien » garde la clé dans un fichier lisible par l'utilisateur seul
    (trousseau macOS à faire quand l'appli Mac sera construite).
  - Clé de récupération : 32 octets aléatoires montrés une fois (base32 de Crockford, groupes de
    4), qui scellent les secrets du compte dans le coffre ; elle ouvre le coffre partout (écran de
    déverrouillage, restauration). Elle n'est pas gardée en clair : « Replace » en fait une
    nouvelle (la maquette disait « Show », impossible sans la garder lisible).
  - Le mot de passe maître peut être retiré (avec une clé de récupération et Windows Hello ou
    « rien » sur cet ordinateur) ; le retrait et une nouvelle clé de récupération passent aux
    autres appareils par la synchronisation. « Set » le remet.
  - Organisation (Security policy › Opening Cleis) : « Master password only », « Master password
    or Windows Hello », « Anything, also nothing » (défaut), et une longueur minimale du mot de
    passe maître (10 par défaut). Droit `Rules` (D-67) pour les changer. Chez le membre, à
    chaque synchronisation : la façon interdite est retirée de l'ordinateur, un mot de passe
    absent ou trop court (vérifié au déverrouillage suivant) est demandé.
  - Reste à essayer sur un vrai Windows Hello (le code Windows est vérifié par clippy, pas
    exécuté ici).

- **D-77 Étiquette de version posée par GitHub Actions** (26/11/2026, demande de l'utilisateur, comme
  Revel) : la session ne peut pas pousser d'étiquette (proxy). Quand l'utilisateur valide une version
  sur son PC, la session pousse `release/vX.Y.Z` sur le commit validé ; `tag.yml` vérifie que la
  version correspond à `Cargo.toml` et que l'étiquette n'existe pas, la pose (auteur Claude, D-41),
  lance `release.yml` sur elle par un `workflow_dispatch` (une étiquette poussée avec `GITHUB_TOKEN`
  ne lance aucun flux, une demande de lancement oui) et retire la branche. Le brouillon reste publié
  par l'utilisateur. Le script `etiquette-vX.Y.Z.bat` reste le secours.

- **D-79 Code PIN de l'organisation** (26/11/2026, demande de l'utilisateur : « un simple pin mis en
  place par l'admin afin d'accéder à l'espace entreprise » ; maquettes 36g, 36h validées). Un seul PIN
  pour l'organisation, fixé par un administrateur (droit « Règles ») et communiqué aux membres.
  Politique : actif ou non, longueur 4 / 6 / 8 chiffres, redemandé après 1 h à 7 jours (8 h par
  défaut), blocage après 3 / 5 / 10 erreurs. Le serveur garde l'empreinte Argon2id du PIN
  (`meta.pin`, jamais envoyée aux applis) ; il le demande à l'ouverture de session (avant la
  vérification en deux étapes, pour ne pas consommer un code) et à tout appel passé le délai
  (`pin_required`) ; il compte les erreurs par membre (`members.pin_fails`) et bloque au seuil
  (`pin_locked`, sessions fermées) jusqu'à « Débloquer le code PIN » d'un administrateur ou
  `cleis-server unlock-pin` (dernier administrateur bloqué). Un PIN donné ouvre l'appareil pour le
  délai (`pin_ok`). Changer de longueur efface le PIN : à redéfinir. L'administrateur qui le fixe
  n'est pas interrogé sur son appareil. Dans l'appli, les éléments, contacts et dossiers de
  l'organisation sont masqués tant que le PIN n'est pas donné ; sa section de la barre latérale
  porte un cadenas et montre la carte du PIN à la place de la liste.
  - Écart à la maquette 36g : une ligne « Définir le code PIN » (case + « Définir ») a été ajoutée,
    la maquette ne montrait pas où saisir le PIN.
  - Limite : la copie locale reste déchiffrable par la clé du coffre ; le PIN protège l'accès au
    serveur (synchronisation) et l'affichage, pas le fichier. Une protection du fichier
    demanderait de sceller les clés des espaces avec un secret remis par le serveur après le PIN.

- **D-78 Où vivent les données d'une organisation** (26/11/2026, demandes de l'utilisateur ;
  maquettes 36f, 36i validées). À la création, trois choix : **serveur sur le NAS** (défaut),
  **serveur sur cet ordinateur** (D-63, existant), **dossier partagé** sans serveur (SharePoint
  synchronisé par l'appli OneDrive, Google shared drive par Google Drive pour ordinateur, partage
  NAS, SFTP). Le coffre personnel garde par défaut un dossier partagé (places D-50, D-65).
  - Serveur sur le NAS : Cleis se connecte en SSH (adresse, port, administrateur, mot de passe,
    jamais gardé) et regarde le système. **QNAP (QTS / QuTS hero)** : paquet **Cleis Server
    (.qpkg)** envoyé par SFTP puis installé par `sh paquet.qpkg` : il apparaît dans l'App Center
    (sans signature QNAP : avertissement affiché par QTS), QTS le démarre avec le NAS ; données sous
    `.qpkg/CleisServer/data`. **Autre Linux** (Synology, serveur) : exécutable statique + service
    systemd (`cleis-server install`). **Docker** seulement s'il est déjà présent et qu'aucune des
    deux voies ne convient : rien n'est ajouté au NAS pour Cleis (demande de l'utilisateur). Le
    NAS de l'utilisateur : QNAP TS-863U-RP, AMD GX-420MC (x86-64, AES-NI), QTS 5.2.11 : paquets
    construits pour x86_64 et arm_64.
  - Dossier partagé : pas de serveur ; chaque appli tient une copie de l'état de l'organisation
    rejouée depuis le dossier. Chaque membre écrit seulement dans son sous-dossier (aucun conflit
    de synchronisation entre deux écrivains) des opérations signées (Ed25519) : les requêtes de
    l'API du serveur, rejouées par toutes les applis dans le même ordre (date, membre, numéro) avec
    les mêmes contrôles de droits que le serveur ; une opération refusée est ignorée partout. Les
    lignes restent scellées comme sur le serveur. Retrait d'un membre : appliqué à la synchro
    suivante, ses espaces rechiffrés ; retirer aussi son accès au dossier (SharePoint, NAS).
    Invitation : l'administrateur publie la clé publique dérivée du code ; le nouveau membre signe
    sa demande avec elle (le code ne passe jamais par le dossier). Le code PIN (D-79) n'y est que
    local (pas de serveur pour compter les erreurs).

- **D-80 Compte** (26/11/2026, maquettes 36b–36d validées). Settings › Account et « Sign in » au
  premier lancement sont l'entrée simple de la place de synchronisation (D-50, D-65) : le fichier
  coffre scellé porte les éléments, les réglages, les adhésions aux organisations, et chaque
  synchronisation est une version. GitHub (dépôt privé + jeton) fonctionne ; Microsoft (OneDrive,
  dossier d'application Graph) et Google (drive.appdata) sont affichés grisés tant que Cleis n'est
  pas enregistré chez eux (« En suspens » 4 ; l'utilisateur : « on verra au moment venu »).
  - Écarts à la maquette 36c : « Ce qui vous suit » est montré par des coches, pas des interrupteurs
    (tout voyage dans le même fichier scellé) ; la liste des ordinateurs viendra avec un registre
    des appareils dans le coffre.

- **D-81 Retours sur la 0.2.2** (27/11/2026, demandes de l'utilisateur). Accent par défaut bronze
  `#977327` (« moins agressif ») : 4:1 sur la page sombre, 3,8:1 sur la claire, donc le même dans les
  deux thèmes ; un coffre qui garde l'ancien ocre par défaut `#e0a93b` passe au bronze
  (`Preferences::upgraded`), une couleur choisie reste. « Renew soon » masqué par défaut dans la barre
  latérale (`DEFAULT_SHORTCUTS`). « New item » devient « New password » (bouton, menu, nom du nouvel
  élément). Ouverture sans rien (D-76) : cadenas ouvert en haut. Survol d'un onglet : la lumière
  s'arrête à 8 px de l'onglet actif, dont le contour déborde chez ses voisins (vérifié à 100, 125 et
  150 %). Réglages : plus de phrase sous un choix (Account, Sync & backups), tout en info-bulle
  (charte §1.5, rappel de l'utilisateur).
  - Constat : le mot de passe maître, l'élément « New Item » vide et les sauvegardes vus après
    l'installation de la 0.2.2 étaient le coffre des essais déjà présent sur le PC
    (`%APPDATA%\matv-studio\Cleis\data`) ; l'installateur ne porte que les programmes. D'où l'absence
    du premier lancement (36d), qui ne paraît que sans coffre.
  - Clic droit sur un élément : fonctionne dans la liste (vérifié par de vrais clics sous X11) ; le
    champ mot de passe du panneau de droite n'avait pas de menu : maquette 37o validée.

## En suspens
1. ~~Serveur de l'entreprise : système~~ (D-63 : tous).
2. Safari : résultat de l'essai (D-34), puis adhésion Apple ou non.
3. Validation de l'étape « mot de passe maître » du premier lancement (D-45).
4. Enregistrement des applications OAuth (Microsoft, Google) pour les places OneDrive / Google
   Drive sans appli de bureau (D-50, D-58) ; GitHub n'en a plus besoin (D-65).
5. Essai du remplissage sur un Mac (Windows validé le 25/11/2026).
6. Code d'invitation court (8 caractères, maquette 03) : demanderait un échange PAKE (D-59).
8. ~~Essai de Windows Hello et Touch ID~~ (retirés, D-71).
7. ~~Méthodes de connexion par e-mail, SMS, Google, Microsoft~~ (D-64 ; restent à essayer avec
   un vrai compte SMTP, une passerelle SMS et des clients OAuth de l'organisation).
9. ~~Onglets de coffre~~ (D-69). L'essai à deux serveurs réels n'est pas demandé pour l'instant.
10 à 14. ~~Choix de fin d'audit~~ (D-71).
15. Format monétaire régional en français (`69,99 €`) : aujourd'hui `€69.99` dans les deux
    langues (D-73).
16. Signature des paquets de mise à jour (D-74) : clé Ed25519 gardée dans les secrets du dépôt,
    à créer par l'utilisateur.
