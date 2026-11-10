# Architecture

## Rôles

| Niveau | Projet | Rôle |
|---|---|---|
| Sèche-serviettes | Sauter Marapi tactile 1750 W (750 W de fluide + 1000 W de soufflerie) | sa carte de commande (écran, touches) choisit consigne, mode et Boost ; sa carte de puissance chauffe d'après sa propre sonde |
| Pont | **ce projet** | un ESP (format D1 mini) sur le bus Linap interne : lit ce que fait l'utilisateur, impose ce que décide Home Assistant |
| Maison | automatisation HA (autre projet) | plannings, présence, délestage |

Ce projet ne contient aucune automatisation HA.

## Solution retenue : un ESP au milieu du bus Linap

**Retenu** (11/2026) : un **MITM** sur la nappe entre la carte de commande et la carte de puissance (protocole : `linap.md`).

- DATA est coupé, GND et VCC traversent. L'ESP joue la carte de puissance face à la carte de commande, et la carte de commande face à la carte de puissance : les deux moitiés ont été validées séparément (`discovery/`).
- **L'écran et les touches d'origine restent** : pas d'interface à refaire.
- **Par défaut, l'ESP relaie** : le radiateur se comporte exactement comme sans lui.
- **Home Assistant peut imposer** une consigne, le Boost ou l'arrêt : l'ESP l'envoie à la carte de puissance à la place de l'état de la carte de commande, jusqu'au prochain changement fait aux touches, qui reprend la main.
- **Limite acceptée : l'écran n'est pas synchronisé.** La carte de commande n'affiche que ce qu'on règle sur ses touches ; un Boost lancé par HA ne s'y voit pas. HA, lui, sait tout : réglages faits aux touches, chauffe en cours.

Écartés :

- **Fil pilote + touche Boost simulée** : une carte qui pilotait le radiateur par son fil pilote 6 ordres et déclenchait le Boost par un faux doigt (photoMOS et condensateur sur l'électrode de la touche). Elle fonctionnait sans rien connaître du radiateur, mais sans retour d'état, et le Boost passait par un artifice. Jamais fabriquée ; schéma, circuit imprimé, firmware et notes sont dans l'historique git (supprimés après le commit `c7eed15`).
- **Contrôleur maison à la place de la carte de commande** : écran toujours juste, mais toute l'interface (touches, affichage, programme) à refaire. Reste le repli si le MITM échoue.

## Entités prévues

| Entité | Type | Rôle |
|---|---|---|
| Consigne, Boost, arrêt lus sur la carte de commande | capteurs | ce que l'utilisateur a réglé aux touches |
| Chauffe en cours | capteur binaire | `C7` de la carte de puissance |
| Mode imposé | `select` | transparent (défaut), consigne imposée, Boost, arrêt |
| Consigne imposée | `number` | consigne envoyée à la carte de puissance en mode imposé |

La carte de commande n'envoie que sa consigne effective : HA voit la consigne Éco quand elle est en Éco, sans le mot « Éco ».

## Sécurité

- **Sans Home Assistant** : l'ESP reste en mode transparent, et y revient s'il a perdu HA depuis 30 min alors qu'un état était imposé. C'est le seul risque à couvrir.
- **Pas de relais de repli** (décidé le 10/11/2026) : si la carte elle-même tombe en panne, le radiateur s'arrête en « Err 10 », sans chauffer, donc sans danger. La carte se branche par des connecteurs : la débrancher et reconnecter la nappe d'origine remet le radiateur dans son état d'usine.
- **Démarrage et mise à jour OTA** : une dizaine de secondes sans dialogue, tolérées (30 s mesurées sans effet côté carte de puissance, plusieurs minutes avant « Err » côté carte de commande).
- **Potentiel du secteur** : alimentée par la nappe, toute la carte est au potentiel du bus, donc du secteur, comme le boîtier de commande. Elle est entièrement enfermée, sans partie touchable ni USB accessible, et se met à jour en OTA uniquement.

## Alimentation

**Décidé** (10/11/2026) : la carte s'alimente sur le **5 V de la nappe**, comme le boîtier de commande. Plus de raccordement au 230 V, ni HLK-PM01, ni fusible, ni varistance.

- Le 5 V de la carte de puissance tient ≈ 160 mA en tout et lâche vers 180 mA (`linap.md`). Le boîtier de commande en prend 40, un D1 mini en Wi-Fi 80 en moyenne : ≈ 120 mA, ≈ 40 mA de marge.
- Validé avec un D1 mini sur la nappe et le boîtier de commande branché : VCC à 5,11 V, plusieurs démarrages du radiateur, navigation et Boost.
- Un condensateur de 470 à 1000 µF près de l'ESP absorbe les pointes d'émission Wi-Fi.
- Pas de marge pour un ESP32, qui consomme davantage, ni pour une bobine de relais : la carte reste sobre.
- Sur chaque voie, pas besoin d'optocoupleur : l'ESP est au potentiel du bus, et le montage du banc suffit (pont diviseur en lecture, NPN en émission).
