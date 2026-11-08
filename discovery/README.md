# Découverte du bus Linap

Comment le protocole du bus Linap a été trouvé, et les outils pour refaire chaque étape. Chaque étape garde son firmware ESPHome, même une fois dépassée ; le montage est un seul banc, dont le schéma évolue d'une révision à l'autre (voir *Évolution du montage*). Ce dossier est le cahier de laboratoire du projet.

| Étape | Dossier | Rôle de l'ESP | Où | État |
|---|---|---|---|---|
| 1 | [`1-sniffer/`](1-sniffer/) | écoute la nappe entre les deux cartes | radiateur | fait, protocole de base décodé |

## Le principe : un ESP flottant, piloté en Wi-Fi

**Le bus n'est pas isolé du secteur** : sa masse est reliée au neutre à travers l'alimentation de la carte de puissance. Tout ce qui se branche sur la nappe pendant que la carte de puissance est raccordée est donc au potentiel du secteur. Un analyseur logique relié au Mac est exclu (on l'a essayé hors secteur : ses captures longues échouaient de toute façon en USB).

D'où le banc :

- un **Wemos D1 mini** (ESP8266) relié à la nappe, qui **flotte** au potentiel du bus ;
- alimenté par une **alimentation de labo à sortie flottante** (sans borne de terre) ;
- piloté et lu **uniquement en Wi-Fi**, par le partage de connexion d'un téléphone posé à côté (ESP, Mac et téléphone au même endroit, signal fort) ;
- une **page web** sur l'ESP (`http://<nom>.local`) pour agir pendant l'essai, et un champ « Note » qui marque les observations dans les logs, au milieu des trames.

## Le banc

Schéma : [`linap-bench.kicad_sch`](linap-bench.kicad_sch), produit par [`generate-schematic.py`](generate-schematic.py).

![Banc Linap](linap-bench.svg)

| Repère | Composant | Rôle |
|---|---|---|
| U1 | Wemos D1 mini (ESP8266, 4 Mo) | écoute, Wi-Fi |
| R1, R2 | 10 kΩ et 15 kΩ | pont diviseur : DATA 5,2 V → 3,1 V sur D2 |
| JP1 | cavalier 2 broches, **ouvert** sur le radiateur | fermé, le 5 V du D1 mini alimente le fil rouge de la nappe : une carte seule, sur le bureau, sans secteur |
| J1 | alimentation de labo réglable à sortie flottante, 5,0 V limitée à 300 mA | alimentation sur le radiateur |
| J2 | pinces ou fils Dupont sur la nappe 3 fils (blanc DATA, rouge VCC, noir GND) | raccordement |
| — | multimètre, téléphone avec partage de connexion | mesures, réseau |

Monté ici avec les résistances du tiroir : R1 = 2 × 10 kΩ en série et R2 = 3 × 10 kΩ en série (20 k / 30 k, même rapport 0,6). Le schéma donne les valeurs standard.

## Étape 0 : mesures au multimètre

Avant tout branchement, nappe débranchée puis rebranchée, disjoncteur coupé pour les résistances : identification des fils (noir GND, rouge VCC 5,22 V, blanc DATA au repos à l'état haut) et **résistance entre GND et le neutre** (850 Ω / 1117 Ω, diodes en test) : c'est elle qui a montré que le bus est au potentiel du secteur.

## Étape 1 : écoute (`1-sniffer/`)

- Montage : J1 alimente, JP1 ouvert, J2 piqué sur la nappe entre les deux cartes, VCC non utilisé.
- Firmware : `linap-sniffer.yaml`. `remote_receiver` journalise chaque trame comme la suite des durées de DATA en µs (positif = bas, négatif = haut), une trame se terminant après 10 ms au repos. Le décodage se fait sur le Mac : [`decode.py`](decode.py) retrouve le débit, lit les octets comme une UART et vérifie la somme de contrôle.
- **Capture guidée** : la page web affiche l'action suivante (liste dans `linap-sniffer-steps.h`) ; « Go » écrit un repère dans les logs, puis un compte à rebours dit quand passer à la suite. Les repères tombent dans le même flux que les trames : plus besoin de chronométrer ses gestes.

Déroulé :

1. Flasher **en USB, avant tout câblage** : `esphome run discovery/1-sniffer/linap-sniffer.yaml`. Vérifier les logs en Wi-Fi, puis **débrancher l'USB pour de bon** (mises à jour en OTA).
2. Vérifier l'alimentation de labo : OL à l'ohmmètre entre sa borne − et les broches de sa prise secteur.
3. Disjoncteur coupé : piquages blanc et noir sur la nappe, alimentation de labo à 5,0 V. Plus rien ne se touche ensuite.
4. `esphome logs discovery/1-sniffer/linap-sniffer.yaml | tee capture.log | discovery/decode.py`, puis réenclencher le disjoncteur et suivre la page.
5. Après coup : `discovery/decode.py capture.log --changes --ignore 75` n'affiche que les trames qui changent, entre les repères.
6. Couper le disjoncteur **avant** de toucher au montage.

Résultat (07/11/2026) : 9 600 bauds 8N1, trames de 5 octets avec XOR, qui parle quand, consigne, Boost, arrêt.

## Évolution du montage

| Révision | Date | Changement | Pour |
|---|---|---|---|
| 1 | 07/11/2026 | D1 mini, pont diviseur sur D2, alimentation de labo ; cavalier JP1 prévu pour alimenter une carte seule sur le bureau | étape 1 |

## Fichiers communs

| Fichier | Rôle |
|---|---|
| `linap-bench.yaml` | paquet ESPHome de toutes les étapes : D1 mini, page web, note, heartbeat |
| `decode.py` | décodage sur le Mac des captures de l'étape 1 |
| `generate-schematic.py` | génère le schéma du banc |
| `linap-bench.kicad_sym` | symbole du D1 mini dessiné comme la carte : broches à leur place physique, vue de dessus, USB en bas (produit par `generate-schematic.py`) |
| `../firmware/network.yaml`, `../firmware/hotspot-wifi.yaml` | Wi-Fi (partage de connexion d'abord), API, OTA ; secrets dans `../firmware/secrets.yaml` |

Schéma : modifier `generate-schematic.py`, le relancer, puis `kicad-cli sch erc` et `kicad-cli sch export svg` sur `linap-bench.kicad_sch`.
