# Cleis logo

Supplied 26/11/2026 (v6 the same day): `cleis-light.svg` (light theme) and `cleis-dark.svg` (dark theme).
`design/tools/logo.sh` regenerates from them:

- `crates/cleis-app/assets/logo-{light,dark}.png` (128 px): the logo inside the app, by theme;
- `crates/cleis-app/assets/tray-32.{png,rgba}`: the notification-area icon (light version);
- `packaging/windows/cleis.ico` (16 to 256 px): the executable, installer and shortcuts
  (light version, the more legible of the two at 16 px and on the dark Windows taskbar).
