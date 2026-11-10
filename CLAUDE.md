# Règles du dépôt

Conventions générales : `../../../COOKBOOK.md`. Ici, seulement ce qui est propre aux cartes électroniques, tiré des schémas de ce projet et de `viessmann-optolink-esphome`.

## Schémas KiCad

- **KiCad 10**, feuille A4, en anglais (textes, notes, étiquettes). Cartouche : titre de la carte, `Plokkke`, en commentaire 1 l'appareil et le module (« Sauter Marapi tactile – Rust on ESP32-C3 »), en commentaire 2 l'avertissement secteur s'il y en a un.
- **Le module ESP est dessiné comme la carte réelle** : un symbole de projet dont les broches sont à leur place physique, vu de dessus, avec le côté USB indiqué. Les **barrettes** qui le reçoivent sont collées au symbole, dans le même bloc, avec une note « plugged into J3 (pins 1-8) and J4 (pins 9-16) ». Sur le circuit imprimé, le module n'est qu'un contour, avec une zone sans cuivre sous l'antenne.
- **Un bloc par fonction** (module et barrettes, alimentation, chaque interface…) : rectangle en tirets gris (`110 110 110`, 0,15 mm), avec un titre en tête qui dit ce que fait le bloc et ses repères (« Supply: fuse, varistor, isolated 5 V »). Câblage à l'intérieur d'un bloc ; entre blocs, des étiquettes locales nommées par le signal (`LINAP_DATA`, `GPIO4`), jamais d'étiquettes globales.
- **Domaines de tension séparés** : basse tension d'un côté, secteur et tout ce qui est à son potentiel de l'autre, séparés par une barrière en tirets rouges (`200 0 0`, 0,8 mm) et titrés (« LOW VOLTAGE (5 V / 3.3 V) », « MAINS 230 V … »). Une note liste les pièces à cheval sur la barrière. Une carte entière au potentiel du secteur est encadrée en rouge, comme le banc (`discovery/`).
- **Couleurs de fils**, avec leur légende en note : brun-rouge = L, bleu = N, orange = +5 V, gris = GND, magenta = 3,3 V, vert = TX, bleu-vert = RX, brun = bus Linap. Une couleur nouvelle s'ajoute à la légende.
- **Notes sur la feuille** pour tout ce qui ne se lit pas sur le schéma : valeur à choisir au banc, pièce non montée et pourquoi, UART inversée, interdiction de l'USB sur secteur.
- **Chaque pièce porte sa référence LCSC** (champ `LCSC`) ; les pièces non montées sont marquées DNP. ERC sans erreur.

## Circuits imprimés

- Empreintes de projet produites par un script à partir des fiches techniques des pièces commandées, pas dessinées à la main.
- Tout ce qui touche au 230 V en traversant. Entre côté secteur et basse tension : 3 mm dans l'air, 6 mm de ligne de fuite (fentes si besoin), dans un fichier de règles `.kicad_dru` ; les pièces d'isolement à cheval sur la frontière.
- DRC avec parité du schéma (`kicad-cli pcb drc --schematic-parity`) : ni erreur, ni liaison manquante.
- Un script d'export produit les Gerbers, la BOM et le CPL JLCPCB, le panier LCSC, le rendu `.svg` du schéma, les rendus dessus, dessous et 3D, et le `.step` pour le boîtier.
