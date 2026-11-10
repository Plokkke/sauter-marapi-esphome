# Découverte du bus Linap

Les outils qui ont servi à décoder le bus Linap, et de quoi refaire chaque essai. Le protocole lui-même est décrit dans [`knowledge/linap.md`](../knowledge/linap.md).

| Étape | Dossier | L'ESP… | Où | Résultat |
|---|---|---|---|---|
| 1 | [`1-sniffer/`](1-sniffer/) | écoute la nappe entre les deux cartes | radiateur | format des trames, consigne, Boost, arrêt |
| 2 | [`2-power-emulator/`](2-power-emulator/) | joue la carte de puissance face à la carte de commande seule | bureau, sans secteur | la carte de commande répond et donne tout ce qu'on fait aux touches, mais ne se laisse pas piloter |
| 3 | [`3-controller-emulator/`](3-controller-emulator/) | joue la carte de commande face à la carte de puissance seule | radiateur | la carte de puissance obéit : consigne, Boost, arrêt |

Les étapes 2 et 3 sont les deux moitiés du futur MITM.

## Le principe : un ESP flottant, piloté en Wi-Fi

**Le bus est au potentiel du secteur** dès que la carte de puissance est raccordée : rien de relié au Mac ou à la terre ne peut s'y brancher. D'où le banc :

- un **Wemos D1 mini** (ESP8266) relié à la nappe, qui **flotte** au potentiel du bus ;
- alimenté par une **alimentation de labo à sortie flottante** sur le radiateur, par l'USB du Mac seulement au bureau, sans secteur ;
- piloté et lu **uniquement en Wi-Fi** (partage de connexion d'un téléphone posé à côté) ;
- une **page web** sur l'ESP (`http://<nom>.local`) pour agir pendant l'essai, et un champ « Note » qui écrit un repère dans les logs, au milieu des trames.

## Le banc

Un seul montage pour les trois étapes. Schéma : [`linap-bench.kicad_sch`](linap-bench.kicad_sch), produit par [`generate-schematic.py`](generate-schematic.py).

![Banc Linap](linap-bench.svg)

| Repère | Composant | Rôle |
|---|---|---|
| U1 | Wemos D1 mini (ESP8266, 4 Mo) | écoute, émission, Wi-Fi |
| R1, R2 | 10 kΩ et 15 kΩ (ou 2 × 10 kΩ et 3 × 10 kΩ en série) | pont diviseur : DATA 5,2 V → 3,1 V sur D2 |
| Q1, R6 | NPN en TO-92 (PN2222, 2N3904, BC547…) et 10 kΩ de base | émission : D8 à l'état haut → DATA à l'état bas. **Brochage** : E-B-C pour PN2222 et 2N3904, C-B-E pour BC547 et BC337 (face plate vers soi) ; à vérifier en test de diode (≈ 0,65 V de la base vers les deux autres pattes) |
| R7 | 4,7 kΩ (ou 2 × 10 kΩ en parallèle) | rappel de DATA au fil rouge, quand la carte de commande n'est pas là pour le faire (étape 3) |
| JP1 | cavalier | fermé, le 5 V du D1 mini alimente la carte de commande par le fil rouge (étape 2) |
| J1 | alimentation de labo à sortie flottante, 5,0 V limitée à 300 mA | étapes 1 et 3 |
| J2 | pinces ou fils Dupont sur la nappe (blanc DATA, rouge VCC, noir GND) | raccordement |

| Étape | Alimentation | JP1 | Q1 | R7 | Secteur |
|---|---|---|---|---|---|
| 1. écoute | J1 | ouvert | bloqué | non | oui |
| 2. fausse carte de puissance | USB | **fermé** | émet | non | **non** |
| 3. fausse carte de commande | J1 | ouvert | émet | **oui** | oui |

Pourquoi un NPN pour émettre : une diode vers une broche à 3,3 V écrêterait DATA au repos sous le seuil haut de l'ESP8266. D8 est tiré à la masse sur le D1 mini au démarrage : Q1 reste bloqué pendant le boot.

## Règles de sécurité sur le radiateur (étapes 1 et 3)

1. Flasher **en USB, avant tout câblage**, puis débrancher l'USB pour de bon : ensuite, mises à jour en OTA seulement.
2. Vérifier l'alimentation de labo : OL à l'ohmmètre entre sa borne − et les broches de sa prise secteur.
3. **Disjoncteur coupé** pour câbler, réenclenché seulement une fois tout en place. Plus rien ne se touche ensuite.
4. Couper le disjoncteur **avant** de toucher au montage.

## Étape 1 : écoute (`1-sniffer/`)

- Branché en dérivation sur la nappe, cartes en place. Le firmware `linap-sniffer.yaml` journalise chaque trame comme une suite de durées (µs) ; [`decode.py`](decode.py) les lit sur le Mac comme une UART et vérifie la somme de contrôle.
- **Capture guidée** : la page affiche l'action à faire (liste dans `linap-sniffer-steps.h`), « Go » écrit un repère dans les logs, un compte à rebours dit quand passer à la suite.

```sh
esphome logs discovery/1-sniffer/linap-sniffer.yaml | tee capture.log | discovery/decode.py
discovery/decode.py capture.log --changes --ignore 75   # après coup : seules les trames qui changent, entre les repères
```

Le sniffer coupe les trames après 10 ms de repos : il voit l'appel de la carte de puissance et la réponse de la carte de commande comme une seule trame `46`.

## Étape 2 : fausse carte de puissance (`2-power-emulator/`)

- Au bureau, **sans secteur** : la carte de commande seule prend son 5 V sur le D1 mini (USB, JP1 fermé).
- `linap-power-emulator.yaml` reproduit le cycle de la vraie carte (`75`, `C7`, `61`, octets espacés comme les siens) et ses octets d'appel `46` / `70`, puis décode les réponses.
- Page `http://linap-power-emulator.local` : état `C7` annoncé, octet d'appel marche/arrêt et délai, adresse d'appel après `C7` (« Probe address »), envoi d'une trame quelconque, dernière réponse `46` décodée.
- [`scan.py`](2-power-emulator/scan.py) automatise les essais par la page : adresses d'appel, trames de la carte de puissance modifiées octet par octet, en-têtes inconnus. Chaque essai est marqué dans les logs.

```sh
esphome logs discovery/2-power-emulator/linap-power-emulator.yaml | tee scan.log
discovery/2-power-emulator/scan.py addresses --host <ip>   # puis known, puis unknown
```

**Résultat** : avec l'octet d'appel, la carte de commande répond à chaque appel et donne sa consigne, son Boost, son arrêt, tels que réglés aux touches ; sans appel, elle se tait. Elle ne répond qu'aux adresses `46` et `70`, et rien de ce qu'on lui envoie ne change son état ni son écran.

## Étape 3 : fausse carte de commande (`3-controller-emulator/`)

- Sur le radiateur, **carte de commande débranchée**, R7 montée, règles de sécurité ci-dessus.
- `linap-controller-emulator.yaml` guette l'octet d'appel après `75`, `C7` et `76` en lisant D2 bit à bit, puis répond 1 bit après avec les 4 octets de l'état choisi sur la page.
- Page `http://linap-controller-emulator.local` : mode (normal, Boost, arrêt), consigne, réponse marche/arrêt, écart entre l'appel et la réponse, dernières trames `C7` et `73`.

**Résultat** : la carte de puissance suit le Boost (soufflerie en marche en 0,3 s), l'arrêt et la consigne, et chauffe d'elle-même quand la consigne dépasse la température de la pièce. Limite du banc : l'ESP8266, occupé par le Wi-Fi, ne sert qu'un appel sur deux. Elle le tolère, mais la carte finale devra répondre à chaque appel.

## Fichiers communs

| Fichier | Rôle |
|---|---|
| `linap-bench.yaml` | paquet ESPHome de toutes les étapes : D1 mini, page web, note, heartbeat (niveau de DATA) |
| `linap-transmitter.yaml` | paquet des émulateurs : émission sur D8 (UART0 matérielle, inversée), écoute sur D2, compteur d'échos, envoi d'une trame |
| `linap-frames.h` | codage et décodage des trames, trames connues, lecture d'un octet d'appel, comptage des échos |
| `decode.py` | décodage sur le Mac des captures de l'étape 1 |
| `generate-schematic.py`, `linap-bench.kicad_sym` | schéma du banc et symbole du D1 mini, dessiné comme la carte (broches à leur place, vue de dessus) |
| `../firmware/network.yaml`, `../firmware/hotspot-wifi.yaml` | Wi-Fi (partage de connexion d'abord), API, OTA ; secrets dans `../firmware/secrets.yaml` |

Pour modifier le banc : changer `generate-schematic.py` (révision, date, note), le relancer, puis `kicad-cli sch erc` et `kicad-cli sch export svg` sur `linap-bench.kicad_sch`. Révisions : 1 (07/11/2026, écoute), 2 (08/11/2026, émission par Q1 et rappel R7) ; les anciennes sont dans `git log -- discovery/linap-bench.kicad_sch`.
