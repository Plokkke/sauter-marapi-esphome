# Fil pilote : signaux et montage

Source : notice Sauter Marapi tactile (`manuals/sauter-marapi-tactile-notice-fr.pdf`, p. 8 et 20, réf. U0708012-D).

## Câble de l'appareil

| Fil | Rôle |
|---|---|
| Marron | phase |
| Bleu | neutre |
| Noir | fil pilote (signal mesuré **par rapport au neutre**) |

Appareil de classe II : **aucune terre**, et le fil noir ne doit jamais être relié à la terre.

## Ordres (fil pilote 6 ordres)

| Ordre | Signal fil pilote / neutre | Alternance positive | Alternance négative |
|---|---|---|---|
| Confort | 0 V (pas de signal) | – | – |
| Confort −1 °C | 230 V pendant 3 s, toutes les 300 s | impulsion | impulsion |
| Confort −2 °C | 230 V pendant 7 s, toutes les 300 s | impulsion | impulsion |
| Éco | 230 V | ✔ | ✔ |
| Hors-gel | demi-alternance négative (−115 V) | – | ✔ |
| Arrêt / délestage | demi-alternance positive (+115 V) | ✔ | – |

- **Confort −1 / −2** : consigne Confort moins 1 ou 2 °C, des paliers intermédiaires.
- **Arrêt** : plus aucune chauffe, même pas le hors-gel. **Délestage** : le même signal, envoyé par un délesteur qui coupe brièvement les radiateurs quand la maison approche de la puissance souscrite.
- **Éco** : consigne Confort moins l'abaissement « Abais. T ECO » (−3,5 °C par défaut, réglable de −1 à −9 °C, notice p. 15).
- **Le Boost (soufflerie 1000 W) n'a pas d'ordre fil pilote** : il ne s'active que par la touche de l'appareil. Voir `backlog.md`.

Délais de prise en compte (notice p. 20) : Confort → Éco ≈ 12 s ; Confort −1/−2 → Confort ≈ 5 min. Hors-gel et délestage sont prioritaires.

**Prérequis côté appareil** : mode **Programme Inactif**, sinon le sèche-serviettes suit son programme interne et ignore le fil pilote. En Confort, il suit la consigne réglée sur son écran.

## Schéma

Source : `hardware/radiator-controller.kicad_sch` (KiCad 10), rendu ci-dessous ; version interactive sur [KiCanvas](https://kicanvas.org/?github=https://github.com/Plokkke/radiator-controller/blob/main/hardware/radiator-controller.kicad_sch).

![Schéma](../hardware/radiator-controller.svg)

Un relais 5 V **à contact repos (NC)** relie la phase au fil pilote : au repos, il envoie 230 V, soit **Éco**. Quand l'ESP l'excite, le contact s'ouvre et deux optotriacs à détection de zéro prennent la main, chacun en série avec une diode qui ne laisse passer qu'une alternance. Couleurs des fils : rouge = phase, bleu = neutre, violet = fil pilote, orange = +5 V, vert = LED des optotriacs, bleu-vert = commande du relais, jaune = bobine du relais ; les croisements sans point vert ne sont pas reliés (ils sont dessinés en pont).

| Ordre | K1 (GPIO14) | U1 (GPIO5) | U2 (GPIO4) |
|---|---|---|---|
| Éco | repos | – | – |
| Confort | excité | – | – |
| Confort −1 / −2 | excité | impulsion | impulsion |
| Hors-gel | excité | – | ✔ |
| Arrêt | excité | ✔ | – |

- Carte hors tension, HLK-PM01 grillé, ESP planté (le chien de garde le redémarre, GPIO à 0) : K1 au repos → **Éco**. La bobine ne consomme (≈ 70 mA) qu'en dehors d'Éco, l'état le plus courant.
- GPIO4, GPIO5 et GPIO14 n'ont pas de rôle au démarrage de l'ESP8266 : pas d'impulsion parasite au boot (K1 reste au repos, donc Éco, pendant le démarrage).
- LED des MOC3043 : (3,3 − 1,3) / 220 ≈ 9 mA (seuil 5 mA max, 12 mA max par GPIO).
- L'entrée fil pilote consomme quelques mA : la sortie des MOC3043 (100 mA) suffit, sans triac de puissance.

## Nomenclature

| Réf. | Composant | Rôle |
|---|---|---|
| Réf. | Composant | Boîtier | Rôle |
|---|---|---|---|
| MOD1 | Wemos D1 mini (ESP8266) sur 2 barrettes femelles 1×8 | CMS | Wi-Fi, ESPHome |
| PS1 | Hi-Link HLK-PM01 (230 V → 5 V, 3 W) | traversant | alimentation isolée, repiquée sur l'arrivée murale |
| F1 | fusible TR5 temporisé 500 mA + support TR5 | traversant | protection du module |
| RV1 | varistance 07D561K (275 V AC, Ø 9 mm) | traversant | surtensions, entre phase et neutre en entrée du HLK |
| U1, U2 | MOC3043M (optotriac, passage par zéro, IFT 5 mA) | DIP-6 | isolation et commutation du fil pilote |
| D1, D2 | 1N4007 | DO-41 | sélection de l'alternance |
| R1, R2 | 220 Ω | 0805 | courant LED des optotriacs |
| K1 | relais SRD-05VDC-SL-C (contact NC) | traversant | repli Éco sans alimentation |
| Q1, R3, D3 | MMBT2222A, 1 kΩ, 1N4148W | SOT-23, 0805, SOD-123 | commande de la bobine de K1 |
| J1 | bornier à vis 5 voies, pas 5,08 mm | traversant | L mur, L sèche-serviettes, fil pilote, N mur, N sèche-serviettes |
| — | boîtier étanche IP44 minimum, presse-étoupes | — | salle de bains |

Tout ce qui touche au 230 V reste en traversant, pour garder de grands écartements entre pastilles.

## Sécurité

- **Tout le côté 230 V est sous tension dès que le disjoncteur est enclenché**, y compris le fil pilote. Couper au tableau avant toute intervention.
- Respecter l'isolement entre 230 V et basse tension sur la carte : ≥ 6 mm (fente ou séparation physique sous les MOC3043 et le HLK-PM01).
- Le module est **flashé en USB avant d'être monté**, puis mis à jour uniquement par OTA. Jamais d'USB branché quand il est raccordé au secteur.
- Le fil pilote et le radiateur sont protégés par le **même différentiel 30 mA** (exigence notice p. 8) : c'est le cas en repiquant l'arrivée murale du sèche-serviettes.
- Ne jamais couper l'alimentation du radiateur pour le piloter : la notice l'interdit (usure des cartes), le fil pilote est fait pour ça.
