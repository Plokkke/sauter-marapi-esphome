# Matériel

Cartes KiCad 10. Les deux cartes dessinées ici sont sur un ESP32-C3 super mini alimenté par un HLK-PM01 ; la carte MITM passera à un D1 mini alimenté par la nappe.

| Carte | Dossier | État | Détails |
|---|---|---|---|
| **Linap** | `linap/` | dessinée en dérivation sur le bus (C3, HLK-PM01, optocoupleurs), à redessiner pour le MITM : D1 mini alimenté par la nappe, deux voies | `knowledge/linap.md`, `knowledge/architecture.md` |
| fil pilote + Boost | `pilot-wire/` | piste mise de côté, jamais fabriquée, gardée comme référence | `knowledge/fil-pilote.md` |

Chaque dossier contient le schéma (`.kicad_sch`, rendu `.svg`), le circuit imprimé (`.kicad_pcb`, rendus `-top.png`, `-bottom.png`, `-3d.png`) et un modèle 3D `.step` pour le boîtier.

## Commun aux cartes

- `lib/` : symbole du C3 super mini, empreintes (`radiator-controller.pretty/`, produites par `lib/footprints.py` à partir des fiches techniques), règles d'isolement (`lib/isolation.kicad_dru`, partagé par lien).
- **C3 super mini** enfiché sur deux barrettes CMS 1×8 (J3, J4) à 15,24 mm d'entraxe, antenne contre le bord. Sur la carte, il n'est qu'un contour, avec une zone sans cuivre sous l'antenne ; dans le schéma, son symbole montre les broches à leur place physique, vu de dessus, USB en haut.
- **Isolement** : entre le côté secteur (230 V et bus Linap) et la basse tension, 3 mm dans l'air et 6 mm de ligne de fuite ; entre conducteurs 230 V, et entre eux et le bus Linap, 2,5 mm. Le DRC (`kicad-cli pcb drc --schematic-parity`) ne trouve ni erreur, ni liaison manquante, ni écart avec le schéma ; restent des avertissements de sérigraphie.

## Commander

`hardware/export-fabrication.sh linap` (ou `pilot-wire`) produit dans `hardware/<carte>/fabrication/` :

- `…-gerbers.zip`, les fichiers de fabrication JLCPCB (la carte fil pilote et sa carte Boost détachable forment un seul contour : vérifier au devis que JLCPCB ne la facture pas comme un panneau) ;
- `…-lcsc-cart.csv`, le panier pour le *BOM Tool* de LCSC : y désactiver les lignes `in stock` (résistances de 220 Ω et 1 kΩ déjà en stock) et, si JLCPCB assemble la carte, les lignes `JLCPCB` ;
- `…-bom-jlcpcb.csv` et `…-cpl.csv`, pour l'assemblage facultatif des pièces CMS par JLCPCB ;
- `…-bom.csv`, la liste complète des composants.

Chaque pièce porte sa référence LCSC dans le schéma (champ `LCSC`) ; `split-bom.py` en tire le panier.
