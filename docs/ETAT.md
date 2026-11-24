# Cleis — état

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
